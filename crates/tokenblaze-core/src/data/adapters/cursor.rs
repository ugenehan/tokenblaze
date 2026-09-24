//! Cursor usage log adapter.
//!
//! Dual strategy:
//! 1. Official Dashboard API when access token is available (preferred, accurate)
//! 2. Local state.vscdb bubble estimate as fallback (estimated)
//!
//! The Dashboard API takes priority; if it succeeds once, we stay on it
//! and stop reading local estimates to avoid double-counting.

use chrono::{DateTime, Local};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

use super::cursor_dashboard;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorIngestMode {
    Dashboard,
    LocalEstimate,
}

impl CursorIngestMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CursorIngestMode::Dashboard => "dashboard",
            CursorIngestMode::LocalEstimate => "localEstimate",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "dashboard" => Some(CursorIngestMode::Dashboard),
            "localEstimate" => Some(CursorIngestMode::LocalEstimate),
            _ => None,
        }
    }
}

pub struct CursorPollResult {
    pub events: Vec<UsageEvent>,
    pub new_watermark: i64,
    pub mode: CursorIngestMode,
    pub detail: String,
}

struct DashboardAttemptState {
    last_attempt: Option<Instant>,
    last_success: Option<Instant>,
}

fn dashboard_state() -> &'static Mutex<DashboardAttemptState> {
    static STATE: OnceLock<Mutex<DashboardAttemptState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(DashboardAttemptState {
            last_attempt: None,
            last_success: None,
        })
    })
}

/// Check Cursor connection state.
pub fn connection_state() -> (SourceConnectionState, String) {
    let has_db = state_vscdb_path().map(|p| p.exists()).unwrap_or(false);
    let has_token = cursor_dashboard::has_access_token();

    if !has_db && !has_token {
        return (
            SourceConnectionState::NotFound,
            "Missing Cursor state.vscdb".to_string(),
        );
    }

    if has_token {
        return (SourceConnectionState::Ok, "Dashboard API ready".to_string());
    }

    // Try to open DB to verify permission
    if let Some(path) = state_vscdb_path() {
        match rusqlite::Connection::open_with_flags(
            &path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        ) {
            Ok(conn) => match conn.busy_timeout(Duration::from_millis(750)) {
                Ok(()) => (
                    SourceConnectionState::Ok,
                    "Local estimate (no Cursor token)".to_string(),
                ),
                Err(error) => (SourceConnectionState::ReadError, error.to_string()),
            },
            Err(e) => (SourceConnectionState::NoPermission, e.to_string()),
        }
    } else {
        (
            SourceConnectionState::NotFound,
            "Cursor not found".to_string(),
        )
    }
}

/// Poll for new Cursor usage events.
pub fn poll(
    watermark_ms: i64,
    start_of_day_ms: i64,
    known_dashboard_ids: &HashSet<String>,
    known_local_ids: &HashSet<String>,
    max_new_events: usize,
    force_dashboard: bool,
    dashboard_was_authoritative: bool,
) -> CursorPollResult {
    // Keep dashboard requests bounded while the monitor polls every few
    // seconds. A successful dashboard fetch remains authoritative between
    // attempts. Once the dashboard has succeeded it stays authoritative even
    // across transient failures, otherwise purged local estimates would be
    // inserted again and double-counted.
    if cursor_dashboard::has_access_token() {
        let now = Instant::now();
        let should_attempt = {
            let state = dashboard_state().lock().unwrap();
            force_dashboard
                || state
                    .last_attempt
                    .map(|last| now.duration_since(last) >= Duration::from_secs(45))
                    .unwrap_or(true)
        };
        if should_attempt {
            dashboard_state().lock().unwrap().last_attempt = Some(now);
            let result = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())
                .and_then(|runtime| {
                    runtime
                        .block_on(cursor_dashboard::fetch_today_events(
                            start_of_day_ms,
                            known_dashboard_ids,
                        ))
                        .map_err(|error| error.to_string())
                });
            match result {
                Ok(mut events) => {
                    dashboard_state().lock().unwrap().last_success = Some(now);
                    events.truncate(max_new_events);
                    return CursorPollResult {
                        events,
                        new_watermark: watermark_ms,
                        mode: CursorIngestMode::Dashboard,
                        detail: "Dashboard API".to_string(),
                    };
                }
                Err(error)
                    if dashboard_was_authoritative
                        || dashboard_state().lock().unwrap().last_success.is_some() =>
                {
                    tracing::warn!(%error, "Cursor Dashboard request failed; retaining authoritative mode");
                    return CursorPollResult {
                        events: Vec::new(),
                        new_watermark: watermark_ms,
                        mode: CursorIngestMode::Dashboard,
                        detail: "Dashboard API (temporarily unavailable)".to_string(),
                    };
                }
                Err(error) => {
                    tracing::warn!(%error, "Cursor Dashboard request failed; using local estimate until first success");
                }
            }
        } else if dashboard_was_authoritative
            || dashboard_state().lock().unwrap().last_success.is_some()
        {
            return CursorPollResult {
                events: Vec::new(),
                new_watermark: watermark_ms,
                mode: CursorIngestMode::Dashboard,
                detail: "Dashboard API (rate limited)".to_string(),
            };
        }
    }

    // Local estimate from state.vscdb
    let local = poll_local(
        watermark_ms,
        start_of_day_ms,
        known_local_ids,
        max_new_events,
    );

    CursorPollResult {
        events: local.events,
        new_watermark: local.new_watermark,
        mode: CursorIngestMode::LocalEstimate,
        detail: "Local estimate".to_string(),
    }
}

struct LocalPollResult {
    events: Vec<UsageEvent>,
    new_watermark: i64,
}

fn poll_local(
    watermark_ms: i64,
    start_of_day_ms: i64,
    known_ids: &HashSet<String>,
    max_new_events: usize,
) -> LocalPollResult {
    let Some(db_path) = state_vscdb_path() else {
        return LocalPollResult {
            events: vec![],
            new_watermark: watermark_ms,
        };
    };

    let Ok(conn) = rusqlite::Connection::open_with_flags(
        &db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    ) else {
        return LocalPollResult {
            events: vec![],
            new_watermark: watermark_ms,
        };
    };
    if let Err(error) = conn.busy_timeout(Duration::from_millis(750)) {
        tracing::debug!(%error, "Unable to configure Cursor database busy timeout");
    }

    // Get today's composers
    let mut composers: Vec<(String, i64)> = Vec::new();
    let mut max_updated = watermark_ms;

    if let Ok(mut stmt) = conn.prepare(
        "SELECT composerId, lastUpdatedAt FROM composerHeaders WHERE lastUpdatedAt >= ? ORDER BY lastUpdatedAt ASC;",
    ) {
        let rows = stmt.query_map(rusqlite::params![start_of_day_ms], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        });

        if let Ok(rows) = rows {
            for row in rows.flatten() {
                if row.0 == "empty-state-draft" {
                    continue;
                }
                if row.1 > max_updated {
                    max_updated = row.1;
                }
                composers.push(row);
            }
        }
    }

    let earliest = DateTime::<chrono::Utc>::from_timestamp_millis(start_of_day_ms)
        .map(|utc| utc.with_timezone(&Local))
        .unwrap_or_else(Local::now);
    let mut events: Vec<UsageEvent> = Vec::new();
    let mut exhausted_all = true;

    for (composer_id, updated_ms) in &composers {
        if events.len() >= max_new_events {
            exhausted_all = false;
            break;
        }
        let before = events.len();
        let batch = bubbles_for_composer(
            &conn,
            composer_id,
            *updated_ms,
            &earliest,
            known_ids,
            max_new_events - events.len(),
        );
        events.extend(batch);
        if events.len() - before >= max_new_events - before {
            exhausted_all = false;
        }
    }

    let new_watermark = if exhausted_all {
        max_updated.max(watermark_ms)
    } else {
        watermark_ms
    };

    LocalPollResult {
        events,
        new_watermark,
    }
}

fn bubbles_for_composer(
    conn: &rusqlite::Connection,
    composer_id: &str,
    updated_ms: i64,
    earliest: &DateTime<Local>,
    known_ids: &HashSet<String>,
    remaining: usize,
) -> Vec<UsageEvent> {
    if remaining == 0 {
        return vec![];
    }

    let pattern = format!("bubbleId:{}:%", composer_id);
    let mut pending_keys: Vec<String> = Vec::new();

    if let Ok(mut stmt) = conn.prepare("SELECT key FROM cursorDiskKV WHERE key LIKE ?;") {
        if let Ok(rows) = stmt.query_map(rusqlite::params![pattern], |row| row.get::<_, String>(0))
        {
            for key in rows.flatten() {
                let bubble_id = key
                    .split(':')
                    .next_back()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| key.clone());
                let event_id = format!("cursor:bubble:v2:{}", bubble_id);
                if !known_ids.contains(&event_id) {
                    pending_keys.push(key);
                    if pending_keys.len() >= remaining {
                        break;
                    }
                }
            }
        }
    }

    if pending_keys.is_empty() {
        return vec![];
    }

    let mut events = Vec::new();
    let fallback_date = DateTime::<chrono::Utc>::from_timestamp_millis(updated_ms)
        .map(|utc| utc.with_timezone(&Local))
        .unwrap_or_else(Local::now);

    if let Ok(mut stmt) = conn.prepare("SELECT value FROM cursorDiskKV WHERE key = ? LIMIT 1;") {
        for key in &pending_keys {
            let result: Result<String, _> =
                stmt.query_row(rusqlite::params![key], |row| row.get(0));
            if let Ok(value) = result {
                if let Some(event) = parse_bubble(key, &value, fallback_date) {
                    if &event.timestamp >= earliest {
                        events.push(event);
                        if events.len() >= remaining {
                            break;
                        }
                    }
                }
            }
        }
    }

    events
}

/// Parse a single bubble JSON into a usage event.
pub fn parse_bubble(key: &str, json: &str, fallback_date: DateTime<Local>) -> Option<UsageEvent> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;

    let bubble_type = val.get("type").and_then(|v| v.as_i64()).unwrap_or(-1);
    if bubble_type != 2 {
        return None;
    }

    let bubble_id = val
        .get("bubbleId")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            key.split(':')
                .next_back()
                .map(|s| s.to_string())
                .unwrap_or_else(|| key.to_string())
        });

    let id = format!("cursor:bubble:v2:{}", bubble_id);

    let created_at = val
        .get("createdAt")
        .and_then(|v| v.as_str())
        .and_then(parse_iso_time)
        .unwrap_or(fallback_date);

    let token_count = val.get("tokenCount");
    let input = token_count
        .and_then(|v| v.get("inputTokens"))
        .and_then(|v| v.as_i64());
    let output = token_count
        .and_then(|v| v.get("outputTokens"))
        .and_then(|v| v.as_i64());

    let mut total = input.unwrap_or(0) + output.unwrap_or(0);
    let mut estimated = false;
    let mut out_val = output;
    let mut in_val = input;

    if total == 0 {
        // Estimate from text + thinking + tools
        let estimate = estimate_tokens(&val);
        if estimate < 24 {
            return None;
        }
        out_val = Some(estimate);
        in_val = None;
        total = estimate;
        estimated = true;
    }

    Some(UsageEvent {
        id,
        source: UsageSource::Cursor,
        timestamp: created_at,
        tokens: total,
        breakdown: UsageBreakdown {
            input: in_val,
            output: out_val,
            cache_read: None,
            cache_write: None,
        },
        file_path: state_vscdb_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default(),
        is_estimated: estimated,
    })
}

/// Estimate token count for bubbles without explicit token counts.
fn estimate_tokens(obj: &serde_json::Value) -> i64 {
    let text = obj.get("text").and_then(|v| v.as_str()).unwrap_or("");

    let mut think_chars = 0;
    if let Some(s) = obj.get("thinking").and_then(|v| v.as_str()) {
        think_chars += s.len();
    } else if let Some(d) = obj.get("thinking").and_then(|v| v.as_object()) {
        if let Ok(data) = serde_json::to_string(d) {
            think_chars += data.len();
        }
    }
    if let Some(blocks) = obj.get("allThinkingBlocks") {
        if let Ok(data) = serde_json::to_string(blocks) {
            think_chars += data.len();
        }
    }

    // Tool / attachment JSON
    let mut tool_chars = 0;
    for field in [
        "toolFormerData",
        "toolResults",
        "codeBlocks",
        "attachedCodeChunks",
        "attachedFileCodeChunksMetadataOnly",
        "capabilityStatuses",
        "aiWebSearchResults",
    ] {
        if let Some(value) = obj.get(field) {
            if let Some(s) = value.as_str() {
                tool_chars += s.len();
            } else if let Ok(data) = serde_json::to_string(value) {
                tool_chars += data.len();
            }
        }
    }

    let prose = (text.len() + think_chars).div_ceil(4);
    let tools = tool_chars / 8; // ~8 chars/token for dense JSON
    let mut total = prose as i64 + tools as i64;

    let is_agentic = obj
        .get("isAgentic")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
        || obj.get("toolFormerData").is_some();

    if is_agentic && total > 0 {
        total = total.max(400);
        total = (total as f64 * 2.2) as i64;
    } else if think_chars > 200 {
        total = (total as f64 * 1.35) as i64;
    }

    total.min(24_000)
}

fn state_vscdb_path() -> Option<PathBuf> {
    cursor_dashboard::state_vscdb_path()
}

fn parse_iso_time(s: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|nd| {
                    nd.and_local_timezone(Local)
                        .single()
                        .unwrap_or_else(Local::now)
                })
        })
}
