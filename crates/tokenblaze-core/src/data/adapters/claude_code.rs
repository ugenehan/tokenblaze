//! Claude Code log adapter.
//!
//! Claude Code stores project logs in `~/.claude/projects/**/*.jsonl`.
//! Each line is a JSON object; lines with `type == "assistant"` have
//! a `message.usage` field with token counts.
//!
//! Claude logs are cumulative per message (streaming updates), so we
//! track the best total per message id and emit deltas.

use chrono::{DateTime, Local};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::data::{SourceConnectionState, UsageBreakdown, UsageEvent, UsageSource};

/// Discover Claude Code log files modified since `modified_since`.
pub fn discover_log_files(modified_since: SystemTime) -> Vec<PathBuf> {
    let home = match home_dir() {
        Some(h) => h,
        None => return Vec::new(),
    };

    let projects_dir = home.join(".claude").join("projects");
    let mut files = Vec::new();
    visit_jsonl_files(&projects_dir, modified_since, &mut files);
    files
}

pub fn discover_log_files_at(root: &Path, modified_since: SystemTime) -> Vec<PathBuf> {
    let mut files = Vec::new();
    visit_jsonl_files(root, modified_since, &mut files);
    files
}

/// Parse a single JSONL line from a Claude Code log.
pub fn parse_line(line: &str, file_path: &str) -> Option<UsageEvent> {
    let val: serde_json::Value = serde_json::from_str(line).ok()?;

    // Check type
    let event_type = val.get("type")?.as_str()?;
    if event_type != "assistant" {
        return None;
    }

    // Get usage from message.usage
    let message = val.get("message")?;
    let usage = message.get("usage")?;

    let input_tokens = usage.get("input_tokens").and_then(|v| v.as_i64());
    let output_tokens = usage.get("output_tokens").and_then(|v| v.as_i64());
    // Anthropic's names describe the operation, so preserve their direction:
    // creation is a cache write and read is a cache read.
    let cache_write = usage
        .get("cache_creation_input_tokens")
        .and_then(|v| v.as_i64());
    let cache_read = usage
        .get("cache_read_input_tokens")
        .and_then(|v| v.as_i64());

    // Claude reports the four buckets independently; do not rely on a
    // provider-specific total that may omit cache tokens.
    let total = input_tokens.unwrap_or(0)
        + output_tokens.unwrap_or(0)
        + cache_write.unwrap_or(0)
        + cache_read.unwrap_or(0);

    if total <= 0 {
        return None;
    }

    let message_id = message
        .get("id")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let timestamp = val
        .get("timestamp")
        .or_else(|| val.get("created_at"))
        .and_then(|v| v.as_str())
        .and_then(parse_iso_time)
        .unwrap_or_else(Local::now);

    let id = message_id
        .map(|value| format!("claude:{}", value))
        .or_else(|| {
            val.get("uuid")
                .and_then(|v| v.as_str())
                .map(|value| format!("claude-uuid:{}", value))
        })
        .unwrap_or_else(|| {
            format!(
                "claude:{}:{}:{}:{}:{}",
                file_path,
                timestamp.timestamp_millis(),
                total,
                input_tokens.unwrap_or(0),
                output_tokens.unwrap_or(0)
            )
        });

    Some(UsageEvent {
        id,
        source: UsageSource::ClaudeCode,
        timestamp,
        tokens: total,
        breakdown: UsageBreakdown {
            input: input_tokens,
            output: output_tokens,
            cache_read,
            cache_write,
        },
        file_path: file_path.to_string(),
        is_estimated: false,
    })
}

/// Check if Claude Code logs are available.
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

    let projects_dir = home.join(".claude").join("projects");
    if !projects_dir.exists() {
        return (
            SourceConnectionState::NotFound,
            "~/.claude/projects not found".to_string(),
        );
    }

    match std::fs::read_dir(&projects_dir) {
        Ok(_) => (SourceConnectionState::Ok, "~/.claude/projects".to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => (
            SourceConnectionState::NoPermission,
            format!("Permission denied: {}", projects_dir.display()),
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
