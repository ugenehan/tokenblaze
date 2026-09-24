//! Pi log adapter.
//!
//! Inflection AI's Pi — reads local usage logs if available.
//! Best-effort adapter; format may vary.

use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

pub fn discover_log_files(modified_since: SystemTime) -> Vec<PathBuf> {
    let root = match pi_sessions_root() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let candidates = [
        root.clone(),
        root.parent()
            .map(|p| p.join("logs"))
            .unwrap_or_else(|| root.join("logs")),
    ];

    let mut files = Vec::new();
    for dir in candidates {
        if dir.exists() {
            visit_jsonl_files(&dir, modified_since, &mut files);
        }
    }
    files
}

pub fn discover_log_files_at(root: &Path, modified_since: SystemTime) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_jsonl_files(root, modified_since, &mut files);
    files
}

pub fn parse_line(line: &str, file_path: &str) -> Option<UsageEvent> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;

    let message = val.get("message").unwrap_or(&val);
    let role = message.get("role").and_then(|v| v.as_str()).unwrap_or("");
    if !role.is_empty() && !matches!(role, "assistant" | "model" | "bot") {
        return None;
    }
    let usage = message.get("usage").or_else(|| val.get("usage"))?;

    let input = value_i64(usage, &["input", "input_tokens", "prompt_tokens"]);
    let output = value_i64(usage, &["output", "output_tokens", "completion_tokens"]);
    let cache_read = value_i64(usage, &["cacheRead", "cache_read_tokens"])
        .unwrap_or(0)
        .max(0);
    let cache_write = value_i64(usage, &["cacheWrite", "cacheWrite1h", "cache_write_tokens"])
        .unwrap_or(0)
        .max(0);
    let tokens = value_i64(usage, &["totalTokens", "total_tokens"])
        .filter(|n| *n > 0)
        .unwrap_or(
            input.unwrap_or(0).max(0) + output.unwrap_or(0).max(0) + cache_read + cache_write,
        );
    if tokens <= 0 {
        return None;
    }

    let timestamp = val
        .get("timestamp")
        .and_then(parse_timestamp)
        .or_else(|| message.get("timestamp").and_then(parse_timestamp))
        .or_else(|| val.get("created_at").and_then(parse_timestamp))
        .unwrap_or_else(Local::now);

    let id_value = message
        .get("id")
        .or_else(|| val.get("id"))
        .and_then(value_string);
    let id = id_value.map(|s| format!("pi:{}", s)).unwrap_or_else(|| {
        format!(
            "pi:{}:{}:{}",
            file_path,
            timestamp.timestamp_millis(),
            tokens
        )
    });

    Some(UsageEvent {
        id,
        source: UsageSource::Pi,
        timestamp,
        tokens,
        breakdown: UsageBreakdown {
            input: input.filter(|n| *n > 0),
            output: output.filter(|n| *n > 0),
            cache_read: (cache_read > 0).then_some(cache_read),
            cache_write: (cache_write > 0).then_some(cache_write),
        },
        file_path: file_path.to_string(),
        is_estimated: false,
    })
}

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

    let sessions = pi_sessions_root().unwrap_or_else(|| home.join(".pi/agent/sessions"));
    if sessions.is_dir() {
        (
            SourceConnectionState::Ok,
            sessions.to_string_lossy().to_string(),
        )
    } else if let Some(agent_dir) = sessions.parent().filter(|p| p.is_dir()) {
        // Match the native client: ~/.pi/agent is enough to report Pi as installed.
        (
            SourceConnectionState::Ok,
            agent_dir.to_string_lossy().to_string(),
        )
    } else {
        (
            SourceConnectionState::NotFound,
            "Pi logs not found".to_string(),
        )
    }
}

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf())
}

fn pi_sessions_root() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("PI_AGENT_DIR") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    home_dir().map(|h| h.join(".pi/agent/sessions"))
}

fn value_i64(value: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_u64().and_then(|n| i64::try_from(n).ok()))
                .or_else(|| v.as_f64().map(|n| n.round() as i64))
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
    })
}

fn value_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_owned)
        .or_else(|| value.as_i64().map(|n| n.to_string()))
        .or_else(|| value.as_u64().map(|n| n.to_string()))
}

fn parse_timestamp(value: &serde_json::Value) -> Option<DateTime<Local>> {
    if let Some(s) = value.as_str() {
        if let Some(dt) = parse_iso_time(s) {
            return Some(dt);
        }
        if let Ok(number) = s.parse::<f64>() {
            return parse_epoch(number);
        }
    }
    value
        .as_i64()
        .map(|n| n as f64)
        .or_else(|| value.as_u64().map(|n| n as f64))
        .or_else(|| value.as_f64())
        .and_then(parse_epoch)
}

fn parse_epoch(value: f64) -> Option<DateTime<Local>> {
    let (seconds, nanos) = if value.abs() >= 10_000_000_000.0 {
        let seconds = (value / 1000.0).floor() as i64;
        let nanos = ((value - seconds as f64 * 1000.0).max(0.0) * 1_000_000.0)
            .round()
            .clamp(0.0, 999_999_999.0) as u32;
        (seconds, nanos)
    } else {
        let seconds = value.floor() as i64;
        let nanos = ((value - seconds as f64) * 1_000_000_000.0)
            .round()
            .clamp(0.0, 999_999_999.0) as u32;
        (seconds, nanos)
    };
    chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, nanos)
        .map(|utc| utc.with_timezone(&Local))
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
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "jsonl" || e == "log" || e == "json")
            .unwrap_or(false)
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_assistant_usage() {
        let line = r#"{"timestamp":"2026-09-17T08:00:00Z","message":{"role":"assistant","id":"m-1","usage":{"input":100,"output":25,"cacheRead":10,"cacheWrite1h":2}}}"#;
        let event = parse_line(line, "/tmp/session.jsonl").unwrap();
        assert_eq!(event.id, "pi:m-1");
        assert_eq!(event.tokens, 137);
        assert_eq!(event.breakdown.input, Some(100));
        assert_eq!(event.breakdown.cache_read, Some(10));
        assert_eq!(event.breakdown.cache_write, Some(2));
    }

    #[test]
    fn parses_fractional_epoch_timestamp() {
        let line = r#"{"id":"m-2","timestamp":1726560000.5,"message":{"role":"assistant","usage":{"input":1,"output":2}}}"#;
        let event = parse_line(line, "/tmp/session.jsonl").unwrap();
        assert_eq!(event.tokens, 3);
        assert_eq!(event.timestamp.timestamp_subsec_millis(), 500);
    }
}
