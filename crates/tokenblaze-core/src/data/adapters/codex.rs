//! Codex log adapter.
//!
//! Codex stores session logs in `~/.codex/sessions/**/*.jsonl`.
//! Each line is a JSON object; we look for `token_usage_record` events
//! with cumulative token counts, converting them to deltas.

use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

/// Discover Codex session log files modified since `modified_since`.
pub fn discover_log_files(modified_since: SystemTime) -> Vec<PathBuf> {
    let home = match home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let sessions_dir = home.join(".codex").join("sessions");
    let mut files = Vec::new();
    visit_jsonl_files(&sessions_dir, modified_since, &mut files);
    files
}

pub fn discover_log_files_at(root: &Path, modified_since: SystemTime) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_jsonl_files(root, modified_since, &mut files);
    files
}

/// Parse a single JSONL line from a Codex session log.
pub fn parse_line(line: &str, file_path: &str) -> Option<UsageEvent> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;

    // Look for token_usage_record type
    let event_type = val.get("type")?.as_str()?;
    if event_type != "token_usage_record" {
        return None;
    }

    // Current Codex session records put usage and identifiers in `payload`.
    // Accept the historical top-level shape as well so existing logs remain
    // readable after an upgrade.
    let payload = val.get("payload").unwrap_or(&val);
    let usage = payload.get("usage").or_else(|| val.get("usage"))?;
    let input_total = token_count(usage, "input_tokens");
    let output = token_count(usage, "output_tokens");
    let cache_read = token_count(usage, "cached_input_tokens")
        .or_else(|| token_count(usage, "cache_read_input_tokens"));
    let cache_write = token_count(usage, "cache_write_input_tokens")
        .or_else(|| token_count(usage, "cache_creation_input_tokens"));
    let total = token_count(usage, "total_tokens")
        .or_else(|| match (input_total, output) {
            (Some(input), Some(output)) => Some(input + output),
            _ => None,
        })?
        .saturating_add(cache_write.unwrap_or(0));
    if total <= 0 {
        return None;
    }
    let input = input_total.map(|value| value.saturating_sub(cache_read.unwrap_or(0)));

    let timestamp = val
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(parse_iso_time)
        .unwrap_or_else(Local::now);

    let id = payload
        .get("response_id")
        .or_else(|| payload.get("message_id"))
        .or_else(|| val.get("response_id"))
        .or_else(|| val.get("message_id"))
        .and_then(value_as_string)
        .map(|value| format!("codex:{}", value))
        .or_else(|| {
            let session = payload
                .get("session_id")
                .or_else(|| val.get("session_id"))
                .and_then(value_as_string)?;
            let ordinal = val
                .get("ordinal")
                .or_else(|| payload.get("ordinal"))
                .and_then(value_as_string)
                .unwrap_or_else(|| timestamp_key(&timestamp));
            Some(format!("codex:{}:{}", session, ordinal))
        })
        .unwrap_or_else(|| {
            format!(
                "codex:{}:{}:{}:{}:{}",
                file_path,
                timestamp_key(&timestamp),
                total,
                input.unwrap_or(0),
                output.unwrap_or(0)
            )
        });

    // Note: Codex logs can be cumulative; we treat each record as a snapshot.
    // Deduplication by id + delta calculation happens in UsageMonitor.

    Some(UsageEvent {
        id,
        source: UsageSource::Codex,
        timestamp,
        tokens: total,
        breakdown: UsageBreakdown {
            input,
            output,
            cache_read,
            cache_write,
        },
        file_path: file_path.to_string(),
        is_estimated: false,
    })
}

fn token_count(value: &serde_json::Value, key: &str) -> Option<i64> {
    value.get(key).and_then(|v| {
        v.as_i64()
            .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
            .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
    })
}

fn value_as_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|n| n.to_string()))
        .or_else(|| value.as_u64().map(|n| n.to_string()))
}

fn timestamp_key(timestamp: &DateTime<Local>) -> String {
    timestamp.timestamp_millis().to_string()
}

/// Check if Codex logs are available.
pub fn connection_state() -> (SourceConnectionState, String) {
    let home = match home_dir() {
        Some(h) => h,
        None => {
            return (
                SourceConnectionState::NotFound,
                "No home directory".to_string(),
            )
        }
    };

    let sessions_dir = home.join(".codex").join("sessions");
    if !sessions_dir.exists() {
        return (
            SourceConnectionState::NotFound,
            "~/.codex/sessions not found".to_string(),
        );
    }

    // Try to read a file to check permissions
    match std::fs::read_dir(&sessions_dir) {
        Ok(_) => (SourceConnectionState::Ok, "~/.codex/sessions".to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => (
            SourceConnectionState::NoPermission,
            format!("Permission denied: {}", sessions_dir.display()),
        ),
        Err(e) => (SourceConnectionState::ReadError, e.to_string()),
    }
}

// ── helpers ────────────────────────────────────────────────────

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf())
}

fn visit_jsonl_files(dir: &Path, modified_since: SystemTime, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_jsonl_files(&path, modified_since, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if modified >= modified_since {
                        out.push(path);
                    }
                }
            }
        }
    }
}

fn parse_iso_time(s: &str) -> Option<DateTime<Local>> {
    // Try ISO 8601 with various formats
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.with_timezone(&Local))
        .or_else(|| {
            // Try without timezone
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .ok()
                .map(|nd| {
                    nd.and_local_timezone(Local)
                        .single()
                        .unwrap_or_else(Local::now)
                })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_current_payload_shape_and_cache_breakdown() {
        let line = r#"{
            "type":"token_usage_record",
            "timestamp":"2026-09-17T08:00:00Z",
            "ordinal":7,
            "payload":{
                "response_id":"resp-1",
                "session_id":"sess-1",
                "usage":{
                    "input_tokens":100,
                    "output_tokens":25,
                    "cached_input_tokens":40,
                    "cache_write_input_tokens":5,
                    "total_tokens":125,
                    "reasoning_output_tokens":10
                }
            }
        }"#;

        let event = parse_line(line, "session.jsonl").expect("usage event");
        assert_eq!(event.id, "codex:resp-1");
        assert_eq!(event.tokens, 130);
        assert_eq!(event.breakdown.input, Some(60));
        assert_eq!(event.breakdown.output, Some(25));
        assert_eq!(event.breakdown.cache_read, Some(40));
        assert_eq!(event.breakdown.cache_write, Some(5));
    }

    #[test]
    fn accepts_legacy_top_level_usage() {
        let line = r#"{
            "type":"token_usage_record",
            "timestamp":"2026-09-17T08:00:00Z",
            "message_id":"legacy-1",
            "usage":{"input_tokens":4,"output_tokens":6}
        }"#;

        let event = parse_line(line, "session.jsonl").expect("usage event");
        assert_eq!(event.id, "codex:legacy-1");
        assert_eq!(event.tokens, 10);
    }
}
