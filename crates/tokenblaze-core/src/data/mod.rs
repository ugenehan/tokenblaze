//! 📊 Usage data layer.
//!
//! Reads AI coding tool usage logs from the local filesystem,
//! stores them in SQLite, and feeds the fire state machine.

pub mod adapters;
pub mod event;
pub mod monitor;
pub mod store;
pub mod snapshot;

pub use event::{
    HourlyUsage, SourceConnectionState, SourceStatus, UsageBreakdown, UsageEvent, UsageSource,
};
pub use monitor::UsageMonitor;
pub use store::UsageStore;
