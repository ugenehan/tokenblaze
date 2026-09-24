//! OpenCode usage adapter.
//!
//! Current OpenCode releases store messages in
//! `~/.local/share/opencode/opencode.db`. The adapter opens that database
//! read-only and records completed assistant messages without reading message
//! parts or content.

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use rusqlite::{Connection, OpenFlags};
use std::path::PathBuf;
use std::time::Duration;

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

pub struct PollResult {
    pub events: Vec<UsageEvent>,
    pub new_watermark: i64,
}

/// Read messages changed after `updated_after_ms`.
pub fn poll(updated_after_ms: i64) -> Result<PollResult> {
    let path = database_path().context("OpenCode database path is unavailable")?;
    let conn = open_read_only(&path)?;
    conn.busy_timeout(Duration::from_millis(500))?;

    let mut statement = conn.prepare(
        r#"
        SELECT id, session_id, time_created, time_updated, data
        FROM message
        WHERE time_updated > ?
        ORDER BY time_updated ASC, id ASC
        "#,
    )?;
    let rows = statement.query_map([updated_after_ms], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
        ))
    })?;

    let path_string = path.to_string_lossy().to_string();
    let mut events = Vec::new();
    let mut new_watermark = updated_after_ms;
    for row in rows {
        let (message_id, session_id, created_ms, updated_ms, data) = row?;
        new_watermark = new_watermark.max(updated_ms);
        if let Some(event) = parse_message(
            &message_id,
            &session_id,
            created_ms,
            updated_ms,
            &data,
            &path_string,
        ) {
            events.push(event);
        }
    }

    Ok(PollResult {
        events,
        new_watermark,
    })
}

pub fn connection_state() -> (SourceConnectionState, String) {
    let Some(path) = database_path() else {
        return (
            SourceConnectionState::Unsupported,
            "OpenCode uses an in-memory database".to_string(),
        );
    };
    if !path.is_file() {
        return (
            SourceConnectionState::NotFound,
            "OpenCode database not found".to_string(),
        );
    }
    if let Err(error) = std::fs::File::open(&path) {
        let state = if error.kind() == std::io::ErrorKind::PermissionDenied {
            SourceConnectionState::NoPermission
        } else {
            SourceConnectionState::ReadError
        };
        return (state, "Cannot read OpenCode database".to_string());
    }

    match open_read_only(&path).and_then(|conn| {
        conn.prepare("SELECT id, session_id, time_created, time_updated, data FROM message LIMIT 0")?;
        Ok(())
    }) {
        Ok(()) => (
            SourceConnectionState::Ok,
            path.to_string_lossy().to_string(),
        ),
        Err(_) => (
            SourceConnectionState::Unsupported,
            "Unsupported OpenCode database schema".to_string(),
        ),
    }
}

fn parse_message(
    message_id: &str,
    session_id: &str,
    created_ms: i64,
    updated_ms: i64,
    encoded: &str,
    file_path: &str,
) -> Option<UsageEvent> {
    let message: serde_json::Value = serde_json::from_str(encoded).ok()?;
    if message.get("role").and_then(|value| value.as_str()) != Some("assistant") {
        return None;
    }

    // OpenCode updates a message while it streams. Waiting for completion
    // prevents an early cumulative snapshot from being stored permanently.
    let completed_ms = message
        .pointer("/time/completed")
        .and_then(value_i64)
        .or_else(|| message.get("finish").filter(|value| !value.is_null()).map(|_| updated_ms))
        .or_else(|| message.get("error").filter(|value| !value.is_null()).map(|_| updated_ms))?;

    let usage = message.get("tokens")?;
    let input = value_at(usage, "input").max(0);
    let output = value_at(usage, "output").max(0);
    let reasoning = value_at(usage, "reasoning").max(0);
    let cache_read = usage.pointer("/cache/read").and_then(value_i64).unwrap_or(0).max(0);
    let cache_write = usage
        .pointer("/cache/write")
        .and_then(value_i64)
        .unwrap_or(0)
        .max(0);
    let total = usage
        .get("total")
        .and_then(value_i64)
        .filter(|value| *value > 0)
        .unwrap_or(input + output + reasoning + cache_read + cache_write);
    if total <= 0 {
        return None;
    }

    let timestamp = timestamp_from_millis(completed_ms)
        .or_else(|| timestamp_from_millis(created_ms))
        .unwrap_or_else(Local::now);
    Some(UsageEvent {
        id: format!("opencode:{}:{}", session_id, message_id),
        source: UsageSource::OpenCode,
        timestamp,
        tokens: total,
        breakdown: UsageBreakdown {
            input: (input > 0).then_some(input),
            output: (output > 0).then_some(output),
            cache_read: (cache_read > 0).then_some(cache_read),
            cache_write: (cache_write > 0).then_some(cache_write),
        },
        file_path: file_path.to_string(),
        is_estimated: false,
    })
}

fn value_at(value: &serde_json::Value, key: &str) -> i64 {
    value.get(key).and_then(value_i64).unwrap_or(0)
}

fn value_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
        .or_else(|| value.as_f64().map(|number| number.round() as i64))
}

fn timestamp_from_millis(value: i64) -> Option<DateTime<Local>> {
    DateTime::<Utc>::from_timestamp_millis(value).map(|utc| utc.with_timezone(&Local))
}

fn open_read_only(path: &PathBuf) -> Result<Connection> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .with_context(|| format!("Failed to open OpenCode database at {}", path.display()))
}

fn database_path() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("OPENCODE_DB") {
        let value = value.trim();
        if value == ":memory:" {
            return None;
        }
        if !value.is_empty() {
            return Some(PathBuf::from(value));
        }
    }
    if let Ok(value) = std::env::var("XDG_DATA_HOME") {
        if !value.trim().is_empty() {
            return Some(PathBuf::from(value).join("opencode").join("opencode.db"));
        }
    }
    directories::UserDirs::new().map(|dirs| {
        dirs.home_dir()
            .join(".local")
            .join("share")
            .join("opencode")
            .join("opencode.db")
    })
}
