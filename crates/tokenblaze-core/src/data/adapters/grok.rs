//! Grok log adapter.
//!
//! Grok (xAI's coding tool) stores usage logs locally.
//! Exact log format may vary — this is a best-effort adapter.

use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

pub fn discover_log_files(modified_since: SystemTime) -> Vec<PathBuf> {
    let root = match grok_home() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let mut files = Vec::new();
    for dir in grok_log_dirs(&root) {
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

    if val.get("sessionUpdate").and_then(|v| v.as_str()) == Some("turn_completed") {
        let usage = val.get("usage").unwrap_or(&val);
        let input_total = value_i64(usage, &["inputTokens", "input_tokens"])
            .unwrap_or(0)
            .max(0);
        let output = value_i64(usage, &["outputTokens", "output_tokens"]).unwrap_or(0);
        let cache_read = value_i64(usage, &["cachedReadTokens", "cache_read_tokens"]).unwrap_or(0);
        let cache_write =
            value_i64(usage, &["cacheCreationTokens", "cache_write_tokens"]).unwrap_or(0);
        let total = value_i64(usage, &["totalTokens", "total_tokens"])
            .filter(|n| *n > 0)
            .unwrap_or(input_total + output)
            .saturating_add(cache_write.max(0));
        if total <= 0 {
            return None;
        }
        let timestamp = timestamp_from(&val, &["timestamp", "ts"]);
        let turn_id = val
            .get("turnId")
            .or_else(|| val.get("promptId"))
            .and_then(value_string)
            .unwrap_or_else(|| format!("{}:{}", timestamp.timestamp_millis(), total));
        return Some(UsageEvent {
            id: format!("grok:turn:{}", turn_id),
            source: UsageSource::Grok,
            timestamp,
            tokens: total,
            breakdown: UsageBreakdown {
                input: Some((input_total - cache_read).max(0)),
                output: Some(output),
                cache_read: (cache_read > 0).then_some(cache_read),
                cache_write: (cache_write > 0).then_some(cache_write),
            },
            file_path: file_path.to_string(),
            is_estimated: false,
        });
    }

    if val.get("msg").and_then(|v| v.as_str()) != Some("shell.turn.inference_done") {
        return None;
    }
    let ctx = val.get("ctx").and_then(|v| v.as_object());
    let prompt = ctx
        .and_then(|o| value_i64_object(o, &["prompt_tokens", "input_tokens"]))
        .unwrap_or(0)
        .max(0);
    let cached = ctx
        .and_then(|o| value_i64_object(o, &["cached_prompt_tokens", "cache_read_tokens"]))
        .unwrap_or(0)
        .clamp(0, prompt);
    let output = ctx
        .and_then(|o| value_i64_object(o, &["completion_tokens", "output_tokens"]))
        .unwrap_or(0)
        .max(0);
    let total = prompt + output;
    if total <= 0 {
        return None;
    }
    let timestamp = timestamp_from(&val, &["ts", "timestamp"]);
    let sid = val
        .get("sid")
        .and_then(value_string)
        .unwrap_or_else(|| "unknown".to_string());
    Some(UsageEvent {
        id: format!(
            "grok:unified:{}:{}:{}",
            sid,
            timestamp.timestamp_millis(),
            total
        ),
        source: UsageSource::Grok,
        timestamp,
        tokens: total,
        breakdown: UsageBreakdown {
            input: Some(prompt - cached),
            output: Some(output),
            cache_read: (cached > 0).then_some(cached),
            cache_write: None,
        },
        file_path: file_path.to_string(),
        is_estimated: false,
    })
}

pub fn connection_state() -> (SourceConnectionState, String) {
    let Some(grok_dir) = grok_home() else {
        return (
            SourceConnectionState::NotFound,
            "No home directory".to_string(),
        );
    };
    if grok_dir.exists()
        || grok_log_dirs(&grok_dir)
            .into_iter()
            .any(|directory| std::fs::read_dir(directory).is_ok())
    {
        (SourceConnectionState::Ok, "~/.grok".to_string())
    } else {
        (
            SourceConnectionState::NotFound,
            "Grok logs not found".to_string(),
        )
    }
}

fn grok_log_dirs(root: &Path) -> Vec<PathBuf> {
    let mut directories = vec![root.join("logs"), root.join("sessions")];
    if let Some(home) = home_dir() {
        directories.push(
            home.join("Library")
                .join("Application Support")
                .join("Grok")
                .join("logs"),
        );
    }
    directories
}

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf())
}

fn grok_home() -> Option<PathBuf> {
    if let Ok(path) = std::env::var("GROK_HOME") {
        if !path.trim().is_empty() {
            return Some(PathBuf::from(path));
        }
    }
    home_dir().map(|h| h.join(".grok"))
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

fn value_i64_object(
    value: &serde_json::Map<String, serde_json::Value>,
    keys: &[&str],
) -> Option<i64> {
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

fn timestamp_from(value: &serde_json::Value, keys: &[&str]) -> DateTime<Local> {
    for key in keys {
        if let Some(raw) = value.get(*key) {
            if let Some(ts) = raw.as_str().and_then(parse_iso_time) {
                return ts;
            }
            if let Some(ms) = raw
                .as_i64()
                .or_else(|| raw.as_u64().and_then(|n| i64::try_from(n).ok()))
                .or_else(|| raw.as_f64().map(|n| n.round() as i64))
                .or_else(|| raw.as_str().and_then(|s| s.parse::<i64>().ok()))
            {
                let seconds = if ms.abs() < 10_000_000_000 {
                    ms
                } else {
                    ms / 1000
                };
                if let Some(utc) = chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0) {
                    return utc.with_timezone(&Local);
                }
            }
        }
    }
    Local::now()
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
    fn parses_turn_completed_usage_and_cache_breakdown() {
        let line = r#"{"sessionUpdate":"turn_completed","turnId":"t-1","timestamp":"2026-09-17T08:00:00Z","usage":{"inputTokens":120,"outputTokens":30,"cachedReadTokens":20,"cacheCreationTokens":4}}"#;
        let event = parse_line(line, "/tmp/updates.jsonl").unwrap();
        assert_eq!(event.id, "grok:turn:t-1");
        assert_eq!(event.tokens, 154);
        assert_eq!(event.breakdown.input, Some(100));
        assert_eq!(event.breakdown.cache_read, Some(20));
        assert_eq!(event.breakdown.cache_write, Some(4));
    }

    #[test]
    fn parses_legacy_unified_inference_with_numeric_timestamp() {
        let line = r#"{"msg":"shell.turn.inference_done","sid":"s-1","ts":1726560000000,"ctx":{"prompt_tokens":80,"cached_prompt_tokens":10,"completion_tokens":20}}"#;
        let event = parse_line(line, "/tmp/unified.jsonl").unwrap();
        assert_eq!(event.id, "grok:unified:s-1:1726560000000:100");
        assert_eq!(event.tokens, 100);
        assert_eq!(event.breakdown.input, Some(70));
        assert_eq!(event.breakdown.cache_read, Some(10));
    }
}
