//! Amp log adapter.
//!
//! Amp (Another AI coding tool — actual name TBD) reads thread files.
//! Each thread file may contain multiple messages with usage data.

use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

/// Discover Amp thread files modified since `modified_since`.
pub fn discover_thread_files(modified_since: SystemTime) -> Vec<PathBuf> {
    let home = match home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let candidates = [
        home.join(".amp").join("threads"),
        home.join(".amp").join("conversations"),
        home.join("Library")
            .join("Application Support")
            .join("Amp")
            .join("threads"),
    ];

    let mut files = Vec::new();
    for dir in candidates {
        if dir.exists() {
            visit_files(&dir, modified_since, &mut files);
        }
    }
    files
}

pub fn discover_thread_files_at(root: &Path, modified_since: SystemTime) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_files(root, modified_since, &mut files);
    files
}

/// Parse a whole thread file into multiple usage events.
pub fn parse_thread(file: &Path) -> Vec<UsageEvent> {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let path_str = file.to_string_lossy().to_string();
    let mut events = Vec::new();

    // Try JSON array format
    if let Ok(messages) = serde_json::from_str::<serde_json::Value>(&content) {
        if let Some(arr) = messages.as_array() {
            for (i, msg) in arr.iter().enumerate() {
                if let Some(evt) = parse_message(msg, &path_str, i) {
                    events.push(evt);
                }
            }
            return events;
        }
    }

    // Try JSONL format
    for (i, line) in content.lines().enumerate() {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(evt) = parse_message(&val, &path_str, i) {
                events.push(evt);
            }
        }
    }

    events
}

fn parse_message(msg: &serde_json::Value, file_path: &str, index: usize) -> Option<UsageEvent> {
    // Only look at assistant messages with usage data
    let role = msg.get("role").and_then(|v| v.as_str()).unwrap_or("");
    if role != "assistant" && role != "model" && role != "bot" {
        // Also try messages without explicit role but with usage data
        if msg.get("usage").is_none() && msg.get("token_usage").is_none() {
            return None;
        }
    }

    let usage = msg.get("usage").or_else(|| msg.get("token_usage"))?;

    let input = usage
        .get("prompt_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| usage.get("input_tokens").and_then(|v| v.as_i64()));
    let output = usage
        .get("completion_tokens")
        .and_then(|v| v.as_i64())
        .or_else(|| usage.get("output_tokens").and_then(|v| v.as_i64()));
    let cache_read = usage
        .get("cache_read_input_tokens")
        .or_else(|| usage.get("cache_read_tokens"))
        .and_then(|v| v.as_i64());
    let cache_write = usage
        .get("cache_creation_input_tokens")
        .or_else(|| usage.get("cache_write_tokens"))
        .and_then(|v| v.as_i64());
    let reported_total = usage
        .get("total_tokens")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    // Prefer the provider total when present, but keep records that expose
    // only a token breakdown (a common shape in Amp exports).
    let total = if reported_total > 0 {
        reported_total
    } else {
        input.unwrap_or(0)
            + output.unwrap_or(0)
            + cache_read.unwrap_or(0)
            + cache_write.unwrap_or(0)
    };

    if total <= 0 {
        return None;
    }

    let timestamp = msg
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(parse_iso_time)
        .or_else(|| {
            msg.get("created_at")
                .and_then(|v| v.as_str())
                .and_then(parse_iso_time)
        })
        .or_else(|| {
            msg.get("time").and_then(|v| v.as_i64()).and_then(|t| {
                DateTime::<chrono::Utc>::from_timestamp(t, 0).map(|utc| utc.with_timezone(&Local))
            })
        })
        .unwrap_or_else(Local::now);

    let id = msg
        .get("id")
        .and_then(|v| v.as_str())
        .map(|s| format!("amp:{}", s))
        .unwrap_or_else(|| format!("amp:{}:{}", file_path, index));

    Some(UsageEvent {
        id,
        source: UsageSource::Amp,
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

    let amp_dir = home.join(".amp");
    if amp_dir.exists() {
        (SourceConnectionState::Ok, "~/.amp".to_string())
    } else {
        (
            SourceConnectionState::NotFound,
            "Amp logs not found".to_string(),
        )
    }
}

fn home_dir() -> Option<PathBuf> {
    directories::UserDirs::new().map(|d| d.home_dir().to_path_buf())
}

fn visit_files(dir: &Path, modified_since: SystemTime, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, modified_since, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e == "json" || e == "jsonl" || e == "md")
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
