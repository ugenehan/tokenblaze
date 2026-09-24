//! Log file adapters for each supported AI coding tool.
//!
//! Each adapter knows how to:
//! 1. Discover its log files on the local filesystem
//! 2. Parse a single JSONL line into a UsageEvent
//! 3. Report connection state (found / permission denied / etc.)

pub mod amp;
pub mod claude_code;
pub mod codex;
pub mod cursor;
pub mod cursor_dashboard;
pub mod grok;
pub mod opencode;
pub mod pi;
