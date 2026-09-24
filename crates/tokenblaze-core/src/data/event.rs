//! Types for usage events and source status.

use serde::{Deserialize, Serialize};
use std::fmt;

/// AI coding tool whose usage we track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum UsageSource {
    ClaudeCode,
    Codex,
    Cursor,
    Grok,
    Pi,
    Amp,
    OpenCode,
}

impl UsageSource {
    pub const ALL: [UsageSource; 7] = [
        UsageSource::ClaudeCode,
        UsageSource::Codex,
        UsageSource::Cursor,
        UsageSource::Grok,
        UsageSource::Pi,
        UsageSource::Amp,
        UsageSource::OpenCode,
    ];

    pub fn index(self) -> usize {
        match self {
            UsageSource::ClaudeCode => 0,
            UsageSource::Codex => 1,
            UsageSource::Cursor => 2,
            UsageSource::Grok => 3,
            UsageSource::Pi => 4,
            UsageSource::Amp => 5,
            UsageSource::OpenCode => 6,
        }
    }

    pub fn from_index(i: usize) -> UsageSource {
        match i {
            0 => UsageSource::ClaudeCode,
            1 => UsageSource::Codex,
            2 => UsageSource::Cursor,
            3 => UsageSource::Grok,
            4 => UsageSource::Pi,
            5 => UsageSource::Amp,
            6 => UsageSource::OpenCode,
            _ => UsageSource::ClaudeCode,
        }
    }

    /// Stable English display name.
    pub fn display_name(self) -> &'static str {
        match self {
            UsageSource::ClaudeCode => "Claude Code",
            UsageSource::Codex => "Codex",
            UsageSource::Cursor => "Cursor",
            UsageSource::Grok => "Grok",
            UsageSource::Pi => "Pi",
            UsageSource::Amp => "Amp",
            UsageSource::OpenCode => "OpenCode",
        }
    }

    /// Key for l10n lookup.
    pub fn label_key(self) -> &'static str {
        match self {
            UsageSource::ClaudeCode => "source.claude_code",
            UsageSource::Codex => "source.codex",
            UsageSource::Cursor => "source.cursor",
            UsageSource::Grok => "source.grok",
            UsageSource::Pi => "source.pi",
            UsageSource::Amp => "source.amp",
            UsageSource::OpenCode => "source.opencode",
        }
    }
}

impl fmt::Display for UsageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.display_name())
    }
}

/// Connection state of a usage data source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceConnectionState {
    Ok,
    NotFound,
    NoPermission,
    Unsupported,
    ReadError,
}

impl SourceConnectionState {
    pub fn label_key(self) -> &'static str {
        match self {
            SourceConnectionState::Ok => "source.state.ok",
            SourceConnectionState::NotFound => "source.state.notFound",
            SourceConnectionState::NoPermission => "source.state.noPermission",
            SourceConnectionState::Unsupported => "source.state.unsupported",
            SourceConnectionState::ReadError => "source.state.readError",
        }
    }
}

/// Token breakdown: input, output, cache reads/writes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UsageBreakdown {
    pub input: Option<i64>,
    pub output: Option<i64>,
    pub cache_read: Option<i64>,
    pub cache_write: Option<i64>,
}

impl UsageBreakdown {
    pub fn billable_total(self) -> i64 {
        self.input.unwrap_or(0)
            + self.output.unwrap_or(0)
            + self.cache_read.unwrap_or(0)
            + self.cache_write.unwrap_or(0)
    }

    pub fn saturating_delta(self, previous: Self) -> Self {
        fn delta(current: Option<i64>, previous: Option<i64>) -> Option<i64> {
            current.map(|value| value.saturating_sub(previous.unwrap_or(0)))
        }
        Self {
            input: delta(self.input, previous.input),
            output: delta(self.output, previous.output),
            cache_read: delta(self.cache_read, previous.cache_read),
            cache_write: delta(self.cache_write, previous.cache_write),
        }
    }
}

/// A single usage event (one turn / one message worth of tokens).
#[derive(Debug, Clone, PartialEq)]
pub struct UsageEvent {
    pub id: String,
    pub source: UsageSource,
    pub timestamp: chrono::DateTime<chrono::Local>,
    pub tokens: i64,
    pub breakdown: UsageBreakdown,
    pub file_path: String,
    pub is_estimated: bool,
}

/// Status of a data source (for the UI).
#[derive(Debug, Clone)]
pub struct SourceStatus {
    pub state: SourceConnectionState,
    pub detail: String,
    pub last_read_at: Option<chrono::DateTime<chrono::Local>>,
    pub today_tokens: i64,
    pub estimated_tokens: i64,
}

/// Hourly usage bucket.
#[derive(Debug, Clone, Copy)]
pub struct HourlyUsage {
    pub hour: u32,
    pub tokens: i64,
}
