//! Usage monitor — polls log files periodically and feeds the fire.
//!
//! Owns the polling loop, incremental file reading, and the data pipeline
//! from log files → events → store → fire state machine.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use crate::data::adapters::{amp, claude_code, codex, cursor, grok, opencode, pi};
use crate::data::UsageStore;
use crate::data::{
    HourlyUsage, SourceConnectionState, SourceStatus, UsageBreakdown, UsageEvent, UsageSource,
};
use crate::fire::FireStateMachine;

pub struct UsageMonitor {
    store: Arc<UsageStore>,
    fire: std::sync::Weak<std::sync::Mutex<FireStateMachine>>,

    statuses: HashMap<UsageSource, SourceStatus>,
    today_tokens: i64,
    today_by_source: [i64; 7],
    today_estimated_by_source: [i64; 7],
    today_hourly: Vec<HourlyUsage>,
    last_seven_days: Vec<crate::data::store::DailyUsage>,
    today_breakdown: UsageBreakdown,
    last_event: Option<UsageEvent>,
    recent_events: Vec<UsageEvent>,

    did_baseline: bool,
    cumulative_best: HashMap<String, (i64, UsageBreakdown)>,
    scan_receiver: Option<Receiver<ScanResult>>,
    rescan_pending: bool,
    rescan_active: bool,
    source_paths: [Option<PathBuf>; 7],

    warm_window: Duration,
}

struct ScanResult {
    events: Vec<UsageEvent>,
    touched: [bool; 7],
    baseline: bool,
    rescan: bool,
}

impl UsageMonitor {
    pub fn new(store: Arc<UsageStore>) -> Self {
        let statuses = UsageSource::ALL
            .iter()
            .map(|&s| {
                (
                    s,
                    SourceStatus {
                        state: SourceConnectionState::NotFound,
                        detail: "—".to_string(),
                        last_read_at: None,
                        today_tokens: 0,
                        estimated_tokens: 0,
                    },
                )
            })
            .collect();

        UsageMonitor {
            store,
            fire: std::sync::Weak::new(),
            statuses,
            today_tokens: 0,
            today_by_source: [0; 7],
            today_estimated_by_source: [0; 7],
            today_hourly: Vec::new(),
            last_seven_days: Vec::new(),
            today_breakdown: UsageBreakdown::default(),
            last_event: None,
            recent_events: Vec::new(),
            did_baseline: false,
            cumulative_best: HashMap::new(),
            scan_receiver: None,
            rescan_pending: false,
            rescan_active: false,
            source_paths: std::array::from_fn(|_| None),
            warm_window: Duration::from_secs(4 * 60),
        }
    }

    pub fn attach_fire(&mut self, fire: &Arc<std::sync::Mutex<FireStateMachine>>) {
        self.fire = Arc::downgrade(fire);
    }

    /// Do an initial baseline scan + warm up the fire from recent events.
    pub fn start(&mut self) {
        // Version migration for Cursor estimates
        if self.store.meta("cursor.estimate.v2").as_deref() != Some("1") {
            self.store.purge_cursor_bubble_v1();
            self.store.set_meta("cursor.estimate.v2", "1");
            self.store.set_meta("cursor.watermark", "0");
        }

        self.reload_stats();
        self.refresh_source_statuses();
        self.warm_from_store();
        self.enqueue_scan(true, false);
    }

    /// Start one periodic scan unless a background scan is already running.
    pub fn tick(&mut self) {
        self.poll();
        self.enqueue_scan(false, false);
    }

    /// Apply a completed background scan without blocking the event loop.
    pub fn poll(&mut self) {
        let result = match self.scan_receiver.as_ref().map(Receiver::try_recv) {
            Some(Ok(result)) => Some(result),
            Some(Err(TryRecvError::Disconnected)) => {
                tracing::warn!("Usage scan worker stopped before returning a result");
                self.scan_receiver = None;
                self.rescan_active = false;
                self.rescan_pending = false;
                None
            }
            Some(Err(TryRecvError::Empty)) | None => None,
        };

        if let Some(result) = result {
            self.scan_receiver = None;
            self.finish_scan(result);
        }

        if self.scan_receiver.is_none() && self.rescan_pending {
            self.rescan_pending = false;
            self.prepare_rescan();
            self.enqueue_scan(true, true);
        }
    }

    /// Force a full rescan from scratch.
    pub fn rescan(&mut self) {
        self.rescan_active = true;
        if self.scan_receiver.is_some() {
            self.rescan_pending = true;
            return;
        }
        self.prepare_rescan();
        self.enqueue_scan(true, true);
    }

    pub fn set_source_path(&mut self, source: UsageSource, path: Option<PathBuf>) {
        self.source_paths[source.index()] = path;
        self.refresh_source_statuses();
        self.rescan();
    }

    pub fn configure_source_paths(&mut self, paths: &[Option<String>; 7]) {
        self.source_paths = std::array::from_fn(|index| paths[index].as_deref().map(PathBuf::from));
    }

    pub fn is_rescanning(&self) -> bool {
        self.rescan_active
    }

    fn prepare_rescan(&mut self) {
        self.did_baseline = false;
        self.cumulative_best.clear();
        self.store.clear_file_cursors();
        self.reload_stats();
        self.refresh_source_statuses();
    }

    pub fn today_tokens(&self) -> i64 {
        self.today_tokens
    }

    pub fn today_by_source(&self) -> [i64; 7] {
        self.today_by_source
    }

    pub fn today_estimated_by_source(&self) -> [i64; 7] {
        self.today_estimated_by_source
    }

    pub fn today_hourly(&self) -> &[HourlyUsage] {
        &self.today_hourly
    }

    pub fn last_seven_days(&self) -> &[crate::data::store::DailyUsage] {
        &self.last_seven_days
    }

    pub fn today_breakdown(&self) -> UsageBreakdown {
        self.today_breakdown
    }

    pub fn statuses(&self) -> &HashMap<UsageSource, SourceStatus> {
        &self.statuses
    }

    pub fn last_event(&self) -> Option<&UsageEvent> {
        self.last_event.as_ref()
    }

    pub fn recent_events(&self) -> &[UsageEvent] {
        &self.recent_events
    }

    // ── internal ─────────────────────────────────────────────

    fn enqueue_scan(&mut self, baseline: bool, rescan: bool) {
        if self.scan_receiver.is_some() {
            return;
        }
        self.refresh_source_statuses();
        let source_available = UsageSource::ALL.map(|src| {
            self.statuses
                .get(&src)
                .map(|s| s.state == SourceConnectionState::Ok)
                .unwrap_or(false)
        });
        let store = self.store.clone();
        let source_paths = self.source_paths.clone();
        let (sender, receiver) = mpsc::channel();
        match thread::Builder::new()
            .name("tokenblaze-usage-scan".to_string())
            .spawn(move || {
                let mut result =
                    scan_in_background(store, source_available, source_paths, baseline);
                result.rescan = rescan;
                let _ = sender.send(result);
            }) {
            Ok(_) => self.scan_receiver = Some(receiver),
            Err(error) => {
                tracing::error!(%error, "Unable to start usage scan worker");
                self.scan_receiver = None;
                if rescan {
                    self.rescan_active = false;
                }
            }
        }
    }

    fn finish_scan(&mut self, result: ScanResult) {
        if result.rescan {
            self.rescan_active = false;
        }
        let warm_cutoff =
            chrono::Local::now() - chrono::Duration::from_std(self.warm_window).unwrap();
        let baseline_pass = result.baseline || !self.did_baseline;

        for event in result.events {
            self.apply_event(event, warm_cutoff, baseline_pass);
        }

        for &src in &UsageSource::ALL {
            if result.touched[src.index()] {
                self.touch_source(src);
            }
        }

        self.reload_stats();
        if baseline_pass {
            self.did_baseline = true;
        }
        self.refresh_source_statuses();
    }

    /// Read a JSONL file incrementally using saved file cursors.
    #[cfg(test)]
    fn read_jsonl_incremental<F>(
        &self,
        file: &std::path::Path,
        from_start: bool,
        parse: F,
    ) -> Vec<UsageEvent>
    where
        F: Fn(&str, &str) -> Option<UsageEvent>,
    {
        read_jsonl_incremental(&self.store, file, from_start, parse)
    }

    fn apply_event(
        &mut self,
        event: UsageEvent,
        warm_cutoff: chrono::DateTime<chrono::Local>,
        baseline_pass: bool,
    ) {
        let mut to_store = event;

        // Claude Code and Codex records can be cumulative snapshots. Convert
        // both totals and every breakdown field to deltas before persistence.
        if matches!(
            to_store.source,
            UsageSource::ClaudeCode | UsageSource::Codex
        ) {
            let (previous_total, previous_breakdown) =
                match self.cumulative_best.get(&to_store.id).copied() {
                    Some(previous) => previous,
                    None => self
                        .store
                        .cumulative_event_totals(&to_store.id)
                        .unwrap_or((0, UsageBreakdown::default())),
                };
            if to_store.tokens <= previous_total {
                return;
            }
            let snapshot_total = to_store.tokens;
            let snapshot_breakdown = to_store.breakdown;
            self.cumulative_best
                .insert(to_store.id.clone(), (snapshot_total, snapshot_breakdown));

            to_store = UsageEvent {
                id: if previous_total == 0 {
                    to_store.id.clone()
                } else {
                    format!("{}#{}", to_store.id, snapshot_total)
                },
                tokens: snapshot_total - previous_total,
                breakdown: snapshot_breakdown.saturating_delta(previous_breakdown),
                ..to_store
            };
        }

        // Insert into store (idempotent)
        if let Ok(true) = self.store.insert_event(&to_store) {
            self.last_event = Some(to_store.clone());
            self.feed_fire(&to_store, warm_cutoff, baseline_pass);
        }
    }

    fn feed_fire(
        &self,
        event: &UsageEvent,
        warm_cutoff: chrono::DateTime<chrono::Local>,
        baseline_pass: bool,
    ) {
        if event.tokens <= 0 {
            return;
        }
        let Some(fire) = self.fire.upgrade() else {
            return;
        };
        let mut fire = fire.lock().unwrap();

        if baseline_pass {
            if event.timestamp >= warm_cutoff {
                let instant = Instant::now()
                    - (chrono::Local::now() - event.timestamp)
                        .to_std()
                        .unwrap_or_default();
                fire.ingest(event.tokens as f64, Some(event.source), instant, false);
            }
        } else {
            fire.ingest(
                event.tokens as f64,
                Some(event.source),
                Instant::now(),
                true,
            );
        }
    }

    fn warm_from_store(&mut self) {
        let cutoff = chrono::Local::now() - chrono::Duration::from_std(self.warm_window).unwrap();
        let recent = self
            .store
            .recent_events_since(cutoff, 500)
            .unwrap_or_default();

        if let Some(fire) = self.fire.upgrade() {
            let mut fire = fire.lock().unwrap();
            for event in &recent {
                let age = chrono::Local::now() - event.timestamp;
                let instant = Instant::now() - age.to_std().unwrap_or_default();
                fire.ingest(event.tokens as f64, Some(event.source), instant, false);
            }

            // Afterglow if something burned earlier today
            if fire.snapshot().phase == crate::fire::FirePhase::Unlit
                || fire.snapshot().phase == crate::fire::FirePhase::Out
            {
                let start = local_start_of_day();
                let today_events = self
                    .store
                    .recent_events_since(start, 5000)
                    .unwrap_or_default();
                if let Some(last) = today_events.last() {
                    let age = chrono::Local::now() - last.timestamp;
                    let horizon = chrono::Duration::hours(2);
                    if age >= chrono::Duration::zero() && age < horizon {
                        let remaining = 0.12_f64
                            .max(1.0 - age.num_seconds() as f64 / horizon.num_seconds() as f64);
                        let instant = Instant::now() - age.to_std().unwrap_or_default();
                        fire.ingest(30_000.0 * remaining, Some(last.source), instant, false);
                    }
                }
            }
        }
    }

    fn reload_stats(&mut self) {
        if let Ok((total, by_source)) = self.store.today_totals() {
            self.today_tokens = total;
            self.today_by_source = by_source;
            self.today_estimated_by_source = self.store.today_estimated_totals().unwrap_or([0; 7]);
            self.today_hourly = self.store.today_hourly().unwrap_or_default();
            self.last_seven_days = self.store.last_seven_days().unwrap_or_default();
            self.today_breakdown = self.store.today_breakdown().unwrap_or_default();
            self.recent_events = self.store.recent_events(5).unwrap_or_default();
            self.last_event = self.recent_events.first().cloned();
            if let Some(fire) = self.fire.upgrade() {
                let mut fire = fire.lock().unwrap();
                fire.update_today_tokens(total, by_source);
            }
        }
    }

    fn refresh_source_statuses(&mut self) {
        let pairs: [(UsageSource, (SourceConnectionState, String)); 7] = [
            (UsageSource::ClaudeCode, claude_code::connection_state()),
            (UsageSource::Codex, codex::connection_state()),
            (UsageSource::Cursor, cursor::connection_state()),
            (UsageSource::Grok, grok::connection_state()),
            (UsageSource::Pi, pi::connection_state()),
            (UsageSource::Amp, amp::connection_state()),
            (UsageSource::OpenCode, opencode::connection_state()),
        ];

        for (source, (state, detail)) in pairs {
            let (state, detail) = if let Some(path) = &self.source_paths[source.index()] {
                if matches!(
                    source,
                    UsageSource::ClaudeCode
                        | UsageSource::Codex
                        | UsageSource::Grok
                        | UsageSource::Pi
                        | UsageSource::Amp
                ) {
                    match std::fs::read_dir(path) {
                        Ok(_) => (SourceConnectionState::Ok, path.display().to_string()),
                        Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => (
                            SourceConnectionState::NoPermission,
                            path.display().to_string(),
                        ),
                        Err(_) => (SourceConnectionState::NotFound, path.display().to_string()),
                    }
                } else {
                    (state, detail)
                }
            } else {
                (state, detail)
            };
            let detail = if source == UsageSource::Cursor {
                self.store.meta("cursor.ingest.detail").unwrap_or(detail)
            } else {
                detail
            };
            let detail = short_path(&detail);
            if let Some(s) = self.statuses.get_mut(&source) {
                s.state = state;
                s.detail = detail;
                s.today_tokens = self.today_by_source[source.index()];
                s.estimated_tokens = self.today_estimated_by_source[source.index()];
            }
        }
    }

    fn touch_source(&mut self, source: UsageSource) {
        if let Some(s) = self.statuses.get_mut(&source) {
            s.last_read_at = Some(chrono::Local::now());
            s.today_tokens = self.today_by_source[source.index()];
            s.estimated_tokens = self.today_estimated_by_source[source.index()];
        }
    }
}

fn scan_in_background(
    store: Arc<UsageStore>,
    source_available: [bool; 7],
    source_paths: [Option<PathBuf>; 7],
    baseline: bool,
) -> ScanResult {
    let since = SystemTime::now() - Duration::from_secs(12 * 3600);
    let start_of_day_ms = start_of_day_ms();
    let watermark = store
        .meta("cursor.watermark")
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    let want = |source: UsageSource| source_available[source.index()];
    let mut events = Vec::new();

    if want(UsageSource::Codex) {
        let files = source_paths[UsageSource::Codex.index()]
            .as_deref()
            .map(|path| codex::discover_log_files_at(path, since))
            .unwrap_or_else(|| codex::discover_log_files(since));
        for file in files {
            events.extend(read_jsonl_incremental(
                &store,
                &file,
                baseline,
                codex::parse_line,
            ));
        }
    }

    if want(UsageSource::ClaudeCode) {
        let files = source_paths[UsageSource::ClaudeCode.index()]
            .as_deref()
            .map(|path| claude_code::discover_log_files_at(path, since))
            .unwrap_or_else(|| claude_code::discover_log_files(since));
        for file in files {
            events.extend(read_jsonl_incremental(
                &store,
                &file,
                baseline,
                claude_code::parse_line,
            ));
        }
    }

    if want(UsageSource::Grok) {
        let files = source_paths[UsageSource::Grok.index()]
            .as_deref()
            .map(|path| grok::discover_log_files_at(path, since))
            .unwrap_or_else(|| grok::discover_log_files(since));
        for file in files {
            events.extend(read_jsonl_incremental(
                &store,
                &file,
                baseline,
                grok::parse_line,
            ));
        }
    }

    if want(UsageSource::Pi) {
        let files = source_paths[UsageSource::Pi.index()]
            .as_deref()
            .map(|path| pi::discover_log_files_at(path, since))
            .unwrap_or_else(|| pi::discover_log_files(since));
        for file in files {
            events.extend(read_jsonl_incremental(
                &store,
                &file,
                baseline,
                pi::parse_line,
            ));
        }
    }

    if want(UsageSource::Amp) {
        let files = source_paths[UsageSource::Amp.index()]
            .as_deref()
            .map(|path| amp::discover_thread_files_at(path, since))
            .unwrap_or_else(|| amp::discover_thread_files(since));
        for file in files {
            let path = file.to_string_lossy().to_string();
            let fingerprint = file.metadata().ok().and_then(|metadata| {
                metadata.modified().ok().map(|modified| {
                    let modified = modified
                        .duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos();
                    format!("amp:{}:{modified}", metadata.len())
                })
            });
            let (_, _, saved_fingerprint) = store.file_cursor_state(&path);
            if fingerprint.is_some() && fingerprint == saved_fingerprint {
                continue;
            }
            events.extend(amp::parse_thread(&file));
            if let Some(fingerprint) = fingerprint.as_deref() {
                let size = file.metadata().map(|metadata| metadata.len()).unwrap_or(0);
                store.set_file_cursor_state(&path, size, None, Some(fingerprint));
            }
        }
    }

    if want(UsageSource::OpenCode) {
        let start_watermark = start_of_day_ms.saturating_sub(1);
        let updated_after = if baseline {
            start_watermark
        } else {
            store
                .meta("opencode.watermark")
                .and_then(|value| value.parse::<i64>().ok())
                .unwrap_or(start_watermark)
                .max(start_watermark)
        };
        match opencode::poll(updated_after) {
            Ok(result) => {
                events.extend(result.events);
                store.set_meta("opencode.watermark", &result.new_watermark.to_string());
            }
            Err(error) => tracing::warn!(%error, "Unable to read OpenCode usage"),
        }
    }

    if want(UsageSource::Cursor) {
        let known_dash = store
            .event_ids_with_prefix("cursor:dash:")
            .unwrap_or_default();
        let known_local = store
            .event_ids_with_prefix("cursor:bubble:v2:")
            .unwrap_or_default();
        let previous_mode = store
            .meta("cursor.ingest.mode")
            .and_then(|mode| cursor::CursorIngestMode::from_str(&mode));
        let result = cursor::poll(
            watermark,
            start_of_day_ms,
            &known_dash,
            &known_local,
            250,
            baseline,
            previous_mode == Some(cursor::CursorIngestMode::Dashboard),
        );
        events.extend(result.events);
        store.set_meta("cursor.watermark", &result.new_watermark.to_string());
        if result.mode == cursor::CursorIngestMode::Dashboard
            && previous_mode != Some(cursor::CursorIngestMode::Dashboard)
        {
            store.purge_cursor_local_estimates();
        }
        store.set_meta("cursor.ingest.mode", result.mode.as_str());
        store.set_meta("cursor.ingest.detail", &result.detail);
    }

    events.sort_by_key(|event| event.timestamp);
    ScanResult {
        events,
        touched: source_available,
        baseline,
        rescan: false,
    }
}

fn read_jsonl_incremental<F>(
    store: &UsageStore,
    file: &std::path::Path,
    from_start: bool,
    parse: F,
) -> Vec<UsageEvent>
where
    F: Fn(&str, &str) -> Option<UsageEvent>,
{
    use std::io::{BufRead, BufReader, Seek, SeekFrom};

    let path_str = file.to_string_lossy().to_string();
    let (cursor_offset, partial, saved_identity) = store.file_cursor_state(&path_str);
    let mut file = match std::fs::File::open(file) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };
    let metadata = file.metadata().ok();
    let file_size = metadata
        .as_ref()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let current_identity = metadata.as_ref().and_then(file_identity);
    let mut offset = cursor_offset;
    let mut partial_line = partial.unwrap_or_default();
    let truncated = offset > file_size;
    let replaced = saved_identity
        .as_ref()
        .zip(current_identity.as_ref())
        .is_some_and(|(saved, current)| saved != current);
    if from_start || truncated || replaced {
        offset = 0;
        partial_line.clear();
    }

    let _ = file.seek(SeekFrom::Start(0));
    let mut reader = BufReader::new(file);
    if offset > 0 && reader.seek(SeekFrom::Start(offset)).is_err() {
        return Vec::new();
    }

    let mut events = Vec::new();
    let mut line_buf = String::new();
    let mut pending = std::mem::take(&mut partial_line);
    loop {
        line_buf.clear();
        match reader.read_line(&mut line_buf) {
            Ok(0) => break,
            Ok(_) => {
                if !pending.is_empty() {
                    pending.push_str(&line_buf);
                    line_buf = std::mem::take(&mut pending);
                }
                let line = line_buf.trim_end_matches('\n').trim_end_matches('\r');
                if line.is_empty() {
                    continue;
                }
                if !line_buf.ends_with('\n') {
                    pending = line_buf.clone();
                    break;
                }
                if let Some(event) = parse(line, &path_str) {
                    events.push(event);
                }
            }
            Err(_) => break,
        }
    }

    partial_line = pending;
    let new_offset = reader.stream_position().unwrap_or(offset);
    store.set_file_cursor_state(
        &path_str,
        new_offset,
        if partial_line.is_empty() {
            None
        } else {
            Some(&partial_line)
        },
        current_identity.as_deref(),
    );
    events
}

#[cfg(unix)]
fn file_identity(metadata: &std::fs::Metadata) -> Option<String> {
    use std::os::unix::fs::MetadataExt;
    Some(format!("unix:{}:{}", metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn file_identity(metadata: &std::fs::Metadata) -> Option<String> {
    metadata.created().ok().and_then(|created| {
        created
            .duration_since(SystemTime::UNIX_EPOCH)
            .ok()
            .map(|duration| format!("created:{}", duration.as_nanos()))
    })
}

fn short_path(path: &str) -> String {
    if let Ok(home) = std::env::var("HOME") {
        if path.starts_with(&home) {
            return format!("~{}", &path[home.len()..]);
        }
    }
    path.to_string()
}

fn start_of_day_ms() -> i64 {
    local_start_of_day().timestamp_millis()
}

fn local_start_of_day() -> chrono::DateTime<chrono::Local> {
    chrono::Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid local time")
        .and_local_timezone(chrono::Local)
        .earliest()
        .unwrap_or_else(chrono::Local::now)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{UsageBreakdown, UsageEvent, UsageSource};
    use std::io::Write;

    fn event(line: &str, path: &str) -> Option<UsageEvent> {
        (line == "complete").then(|| UsageEvent {
            id: "monitor-test-event".to_string(),
            source: UsageSource::Codex,
            timestamp: chrono::Local::now(),
            tokens: 10,
            breakdown: UsageBreakdown {
                input: Some(7),
                output: Some(3),
                ..UsageBreakdown::default()
            },
            file_path: path.to_string(),
            is_estimated: false,
        })
    }

    #[test]
    fn resumes_saved_partial_line_once() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let suffix = format!("tokenblaze-monitor-{}-{}.jsonl", std::process::id(), nonce);
        let path = std::env::temp_dir().join(suffix);
        let path_str = path.to_string_lossy().to_string();
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"complete").unwrap();

        let store = Arc::new(UsageStore::open_in_memory().unwrap());
        let monitor = UsageMonitor::new(store.clone());
        assert!(monitor
            .read_jsonl_incremental(&path, false, event)
            .is_empty());
        assert_eq!(store.file_cursor(&path_str).1.as_deref(), Some("complete"));

        drop(file);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"\n").unwrap();
        drop(file);

        let events = monitor.read_jsonl_incremental(&path, false, event);
        assert_eq!(events.len(), 1);
        assert_eq!(store.file_cursor(&path_str).1, None);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unchanged_unterminated_line_does_not_restart_from_zero() {
        let path = std::env::temp_dir().join(format!(
            "tokenblaze-monitor-no-rewind-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"complete").unwrap();
        let path_str = path.to_string_lossy().to_string();
        let store = UsageStore::open_in_memory().unwrap();

        assert!(read_jsonl_incremental(&store, &path, false, event).is_empty());
        let first = store.file_cursor(&path_str);
        assert!(read_jsonl_incremental(&store, &path, false, event).is_empty());
        assert_eq!(store.file_cursor(&path_str), first);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn replacement_identity_restarts_cursor() {
        let path = std::env::temp_dir().join(format!(
            "tokenblaze-monitor-rotation-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, b"complete\n").unwrap();
        let path_str = path.to_string_lossy().to_string();
        let store = UsageStore::open_in_memory().unwrap();
        store.set_file_cursor_state(&path_str, 5, None, Some("different-file"));

        let events = read_jsonl_incremental(&store, &path, false, event);
        assert_eq!(events.len(), 1);
        let _ = std::fs::remove_file(path);
    }
}
