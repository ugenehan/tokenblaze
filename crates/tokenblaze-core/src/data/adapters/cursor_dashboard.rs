//! Cursor Dashboard API client.
//!
//! Cursor exposes usage through its Connect RPC endpoint. The client keeps
//! credentials local, pages through today's events, and converts the response
//! into the same event type used by the local log adapters.

use chrono::{DateTime, Local};
use serde_json::Value;
use std::collections::HashSet;
use std::time::Duration;

use crate::data::{UsageBreakdown, UsageEvent, UsageSource};

const ENDPOINT: &str = "https://api2.cursor.sh/aiserver.v1.DashboardService/GetFilteredUsageEvents";
const PAGE_SIZE: usize = 100;
const MAX_PAGES: usize = 20;

#[derive(Debug)]
pub enum FetchError {
    Unauthorized,
    Network(String),
    Parse(String),
}

impl std::fmt::Display for FetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FetchError::Unauthorized => write!(f, "Unauthorized — token invalid or expired"),
            FetchError::Network(s) => write!(f, "Network error: {}", s),
            FetchError::Parse(s) => write!(f, "Parse error: {}", s),
        }
    }
}

impl std::error::Error for FetchError {}

/// Check if a usable access token can be found locally.
pub fn has_access_token() -> bool {
    get_access_token().is_some()
}

/// Read a token from the environment, Cursor's state database, or keyring.
/// The returned value is never logged or included in diagnostics.
pub fn get_access_token() -> Option<String> {
    if let Ok(token) = std::env::var("CURSOR_ACCESS_TOKEN") {
        let token = token.trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }
    if let Some(token) = discover_token_from_state_vscdb() {
        return Some(token);
    }

    // Check TokenBlaze's key first, then Cursor's own credential names.
    for (service, account) in [
        ("tokenblaze", "cursor-dashboard-token"),
        ("cursor-access-token", "cursor-access-token"),
        ("cursor-user", "accessToken"),
    ] {
        if let Ok(entry) = keyring::Entry::new(service, account) {
            if let Ok(token) = entry.get_password() {
                let token = token.trim();
                if !token.is_empty() {
                    return Some(token.to_string());
                }
            }
        }
    }
    None
}

/// Fetch today's usage events from the official Cursor Dashboard API.
pub async fn fetch_today_events(
    start_of_day_ms: i64,
    known_ids: &HashSet<String>,
) -> Result<Vec<UsageEvent>, FetchError> {
    let token = get_access_token().ok_or(FetchError::Unauthorized)?;
    let client = reqwest::Client::builder()
        .user_agent("cursor-agent/2026.08.11")
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let mut events = Vec::new();
    let mut total_count = None;
    for page in 1..=MAX_PAGES {
        let body = serde_json::json!({
            "startDate": start_of_day_ms.to_string(),
            "endDate": chrono::Local::now().timestamp_millis().to_string(),
            "page": page,
            "pageSize": PAGE_SIZE,
        });
        let response = client
            .post(ENDPOINT)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Connect-Protocol-Version", "1")
            .bearer_auth(&token)
            .json(&body)
            .send()
            .await
            .map_err(|e| FetchError::Network(e.to_string()))?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED
            || response.status() == reqwest::StatusCode::FORBIDDEN
        {
            return Err(FetchError::Unauthorized);
        }
        if !response.status().is_success() {
            return Err(FetchError::Network(format!("HTTP {}", response.status())));
        }

        let payload: Value = response
            .json()
            .await
            .map_err(|e| FetchError::Parse(e.to_string()))?;
        total_count = total_count.or_else(|| value_usize(&payload, &["totalUsageEventsCount"]));
        let rows = payload
            .get("usageEventsDisplay")
            .and_then(Value::as_array)
            .or_else(|| payload.as_array())
            .ok_or_else(|| FetchError::Parse("missing usageEventsDisplay".to_string()))?;

        if rows.is_empty() {
            break;
        }
        for row in rows {
            if let Some(event) = parse_dashboard_row(row, known_ids) {
                if event.timestamp.timestamp_millis() >= start_of_day_ms {
                    events.push(event);
                }
            }
        }
        if rows.len() < PAGE_SIZE || total_count.is_some_and(|n| page * PAGE_SIZE >= n) {
            break;
        }
    }
    Ok(events)
}

fn parse_dashboard_row(row: &Value, known_ids: &HashSet<String>) -> Option<UsageEvent> {
    let usage = row.get("tokenUsage").unwrap_or(row);
    let input = value_i64(usage, &["inputTokens", "input"]);
    let output = value_i64(usage, &["outputTokens", "output"]);
    let cache_write = value_i64(usage, &["cacheWriteTokens", "cacheWrite"]);
    let cache_read = value_i64(usage, &["cacheReadTokens", "cacheRead"]);
    // Cursor exposes the four billable buckets independently. Summing them
    // avoids trusting provider-specific total fields that may omit caches.
    let total = input.unwrap_or(0)
        + output.unwrap_or(0)
        + cache_write.unwrap_or(0)
        + cache_read.unwrap_or(0);
    if total <= 0 {
        return None;
    }

    let timestamp_ms = value_timestamp_ms(row.get("timestamp").or_else(|| row.get("createdAt"))?)?;
    let conversation = row
        .get("conversationId")
        .or_else(|| row.get("conversation_id"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let model = row.get("model").and_then(Value::as_str).unwrap_or("cursor");
    let id = format!(
        "cursor:dash:{}:{}:{}:{}:{}:{}:{}",
        conversation,
        timestamp_ms,
        model,
        input.unwrap_or(0),
        output.unwrap_or(0),
        cache_write.unwrap_or(0),
        cache_read.unwrap_or(0)
    );
    if known_ids.contains(&id) {
        return None;
    }

    Some(UsageEvent {
        id,
        source: UsageSource::Cursor,
        timestamp: DateTime::<chrono::Utc>::from_timestamp_millis(timestamp_ms)?
            .with_timezone(&Local),
        tokens: total,
        breakdown: UsageBreakdown {
            input,
            output,
            cache_read,
            cache_write,
        },
        file_path: "cursor://dashboard".to_string(),
        is_estimated: false,
    })
}

fn value_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value
            .get(*key)
            .and_then(|v| v.as_i64().or_else(|| v.as_str()?.parse().ok()))
    })
}

fn value_usize(value: &Value, keys: &[&str]) -> Option<usize> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| {
            v.as_u64()
                .map(|n| n as usize)
                .or_else(|| v.as_str()?.parse().ok())
        })
    })
}

fn value_timestamp_ms(value: &Value) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(if number < 10_000_000_000 {
            number * 1000
        } else {
            number
        });
    }
    let text = value.as_str()?;
    if let Ok(number) = text.parse::<i64>() {
        return Some(if number < 10_000_000_000 {
            number * 1000
        } else {
            number
        });
    }
    parse_iso_time(text).map(|dt| dt.timestamp_millis())
}

fn parse_iso_time(s: &str) -> Option<DateTime<Local>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
        .or_else(|| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .and_then(|nd| nd.and_local_timezone(Local).single())
        })
}

/// Try to extract Cursor's token from User/globalStorage/state.vscdb.
pub fn discover_token_from_state_vscdb() -> Option<String> {
    let state_db = state_vscdb_path()?;
    if !state_db.exists() {
        return None;
    }
    let conn = rusqlite::Connection::open_with_flags(
        &state_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .ok()?;

    // Current Cursor builds use ItemTable; older builds used cursorDiskKV.
    for (table, keys) in [
        (
            "ItemTable",
            &["cursorAuth/accessToken", "cursorAuth/accessTokenV2"][..],
        ),
        (
            "cursorDiskKV",
            &[
                "cursor.auth.token",
                "cursorAuthToken",
                "auth.token",
                "accessToken",
            ][..],
        ),
    ] {
        for key in keys {
            let sql = format!("SELECT value FROM {} WHERE key = ? LIMIT 1", table);
            if let Ok(value) =
                conn.query_row(&sql, rusqlite::params![key], |row| row.get::<_, String>(0))
            {
                let value = value.trim();
                if !value.is_empty() && value.len() > 20 {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

/// Path to Cursor's state.vscdb file.
pub fn state_vscdb_path() -> Option<std::path::PathBuf> {
    let home = directories::UserDirs::new()?.home_dir().to_path_buf();
    #[cfg(target_os = "macos")]
    let path = home.join("Library/Application Support/Cursor/User/globalStorage/state.vscdb");
    #[cfg(target_os = "windows")]
    let path = home.join("AppData/Roaming/Cursor/User/globalStorage/state.vscdb");
    #[cfg(target_os = "linux")]
    let path = home.join(".config/Cursor/User/globalStorage/state.vscdb");
    Some(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_id_is_stable_and_deduplicated() {
        let row = serde_json::json!({
            "conversationId": "c1",
            "timestamp": 1_700_000_000_000_i64,
            "model": "gpt-4",
            "tokenUsage": {"inputTokens": 2, "outputTokens": 3, "totalTokens": 1}
        });
        let event = parse_dashboard_row(&row, &HashSet::new()).unwrap();
        assert_eq!(event.tokens, 5);
        assert!(parse_dashboard_row(&row, &HashSet::from([event.id])).is_none());
    }

    #[test]
    fn timestamp_accepts_seconds_and_iso() {
        assert_eq!(
            value_timestamp_ms(&Value::from(1_700_000_000_i64)),
            Some(1_700_000_000_000)
        );
        assert!(value_timestamp_ms(&Value::from("2024-01-01T00:00:00Z")).is_some());
    }
}
