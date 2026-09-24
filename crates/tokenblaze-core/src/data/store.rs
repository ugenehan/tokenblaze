//! Local SQLite storage for usage events.
//!
//! Stores events, file cursors (for incremental log reading),
//! and metadata key-value pairs.
//!

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Local, Timelike, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::PathBuf;
use std::sync::Mutex;

use super::{HourlyUsage, UsageBreakdown, UsageEvent, UsageSource};

fn local_start_of_day() -> DateTime<Local> {
    Local::now()
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("midnight is a valid local time")
        .and_local_timezone(Local)
        .earliest()
        .unwrap_or_else(Local::now)
}

pub struct UsageStore {
    db: Mutex<Connection>,
    /// In-memory cache of known event IDs to avoid SELECT-per-event.
    known_ids: Mutex<std::collections::HashSet<String>>,
}

#[derive(Debug, Clone)]
pub struct DailyUsage {
    pub date: String,
    pub tokens: i64,
}

impl UsageStore {
    /// Open (or create) the usage database in the user's app support dir.
    pub fn open() -> Result<Self> {
        let path = Self::db_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&path)
            .with_context(|| format!("Failed to open SQLite database at {}", path.display()))?;

        Self::configure_connection(&conn)?;

        let store = UsageStore {
            db: Mutex::new(conn),
            known_ids: Mutex::new(std::collections::HashSet::new()),
        };
        store.migrate()?;
        store.warm_id_cache()?;
        Ok(store)
    }

    /// Open an in-memory database (for testing).
    #[cfg(test)]
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        Self::configure_connection(&conn)?;
        let store = UsageStore {
            db: Mutex::new(conn),
            known_ids: Mutex::new(std::collections::HashSet::new()),
        };
        store.migrate()?;
        Ok(store)
    }

    fn db_path() -> Result<PathBuf> {
        let dirs = directories::ProjectDirs::from("ai", "createfun", "TokenBlaze")
            .context("Failed to determine app support directory")?;
        Ok(dirs.data_dir().join("usage.sqlite"))
    }

    fn configure_connection(conn: &Connection) -> Result<()> {
        // `journal_mode` returns the selected mode, so use execute_batch instead
        // of execute, which rejects statements that return rows.
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;",
        )?;
        Ok(())
    }

    fn migrate(&self) -> Result<()> {
        let db = self.db.lock().unwrap();

        db.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS usage_events (
                id TEXT PRIMARY KEY NOT NULL,
                source TEXT NOT NULL,
                timestamp REAL NOT NULL,
                tokens INTEGER NOT NULL,
                input_tokens INTEGER,
                output_tokens INTEGER,
                cache_read INTEGER,
                cache_write INTEGER,
                file_path TEXT,
                is_estimated INTEGER NOT NULL DEFAULT 0,
                inserted_at REAL NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_events_ts ON usage_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_source_ts ON usage_events(source, timestamp);
            CREATE INDEX IF NOT EXISTS idx_events_file_path ON usage_events(file_path);

            CREATE TABLE IF NOT EXISTS file_cursors (
                path TEXT PRIMARY KEY NOT NULL,
                byte_offset INTEGER NOT NULL,
                partial_line TEXT,
                file_identity TEXT,
                updated_at REAL NOT NULL
            );

            CREATE TABLE IF NOT EXISTS meta (
                key TEXT PRIMARY KEY NOT NULL,
                value TEXT NOT NULL
            );
            "#,
        )?;

        let has_file_identity = {
            let mut statement = db.prepare("PRAGMA table_info(file_cursors);")?;
            let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
            let has_file_identity = columns
                .filter_map(Result::ok)
                .any(|column| column == "file_identity");
            drop(statement);
            has_file_identity
        };
        if !has_file_identity {
            db.execute(
                "ALTER TABLE file_cursors ADD COLUMN file_identity TEXT;",
                [],
            )?;
        }

        Ok(())
    }

    fn warm_id_cache(&self) -> Result<()> {
        let db = self.db.lock().unwrap();
        let mut cache = self.known_ids.lock().unwrap();
        let cutoff = (Local::now() - Duration::days(90)).timestamp_millis() as f64 / 1000.0;
        let mut stmt = db.prepare("SELECT id FROM usage_events WHERE inserted_at >= ?;")?;
        let rows = stmt.query_map(params![cutoff], |row| row.get::<_, String>(0))?;
        for id in rows {
            cache.insert(id?);
        }
        Ok(())
    }

    // ── Events ──────────────────────────────────────────────

    /// Insert a usage event. Returns true if it was new (not duplicate).
    pub fn insert_event(&self, event: &UsageEvent) -> Result<bool> {
        // Fast path: check memory cache first
        {
            let cache = self.known_ids.lock().unwrap();
            if cache.contains(&event.id) {
                return Ok(false);
            }
        }

        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            r#"
            INSERT OR IGNORE INTO usage_events
            (id, source, timestamp, tokens, input_tokens, output_tokens,
             cache_read, cache_write, file_path, is_estimated, inserted_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )?;

        let now = Local::now().timestamp_millis() as f64 / 1000.0;
        let changes = stmt.execute(params![
            event.id,
            event.source.as_ref_str(),
            event.timestamp.timestamp_millis() as f64 / 1000.0,
            event.tokens,
            event.breakdown.input,
            event.breakdown.output,
            event.breakdown.cache_read,
            event.breakdown.cache_write,
            event.file_path,
            event.is_estimated as i32,
            now,
        ])?;

        let inserted = changes > 0;
        if inserted {
            let mut cache = self.known_ids.lock().unwrap();
            cache.insert(event.id.clone());
        }

        Ok(inserted)
    }

    /// Reconstruct the latest cumulative snapshot stored as one base event
    /// followed by `#<snapshot total>` delta events.
    pub fn cumulative_event_totals(&self, base_id: &str) -> Result<(i64, UsageBreakdown)> {
        let db = self.db.lock().unwrap();
        let (tokens, input, output, cache_read, cache_write) = db.query_row(
            r#"
            SELECT
                COALESCE(SUM(tokens), 0),
                COALESCE(SUM(input_tokens), 0),
                COALESCE(SUM(output_tokens), 0),
                COALESCE(SUM(cache_read), 0),
                COALESCE(SUM(cache_write), 0)
            FROM usage_events
            WHERE id = ?1 OR substr(id, 1, length(?1) + 1) = ?1 || '#'
            "#,
            params![base_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )?;
        Ok((
            tokens,
            UsageBreakdown {
                input: (input > 0).then_some(input),
                output: (output > 0).then_some(output),
                cache_read: (cache_read > 0).then_some(cache_read),
                cache_write: (cache_write > 0).then_some(cache_write),
            },
        ))
    }

    /// Get all event IDs that start with a prefix (for Cursor batch dedup).
    pub fn event_ids_with_prefix(&self, prefix: &str) -> Result<std::collections::HashSet<String>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT id FROM usage_events WHERE id LIKE ?;")?;
        let pattern = format!("{}%", prefix);
        let rows = stmt.query_map(params![pattern], |row| row.get::<_, String>(0))?;

        let mut ids = std::collections::HashSet::new();
        for id in rows {
            let id = id?;
            ids.insert(id.clone());
            let mut cache = self.known_ids.lock().unwrap();
            cache.insert(id);
        }
        Ok(ids)
    }

    /// Today's total tokens, grouped by source.
    pub fn today_totals(&self) -> Result<(i64, [i64; 7])> {
        let start = local_start_of_day().timestamp_millis() as f64 / 1000.0;

        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            r#"
            SELECT source, SUM(tokens) FROM usage_events
            WHERE timestamp >= ?
            GROUP BY source
            "#,
        )?;

        let mut by_source = [0i64; 7];
        let mut total = 0i64;

        let rows = stmt.query_map(params![start], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            let (src_str, tokens) = row?;
            if let Some(src) = UsageSource::from_str(&src_str) {
                by_source[src.index()] = tokens;
                total += tokens;
            }
        }

        Ok((total, by_source))
    }

    /// Today's estimated token totals, grouped by source.
    pub fn today_estimated_totals(&self) -> Result<[i64; 7]> {
        let start = local_start_of_day().timestamp_millis() as f64 / 1000.0;
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT source, SUM(tokens) FROM usage_events WHERE timestamp >= ? AND is_estimated != 0 GROUP BY source",
        )?;
        let mut by_source = [0i64; 7];
        let rows = stmt.query_map(params![start], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows {
            let (source, tokens) = row?;
            if let Some(source) = UsageSource::from_str(&source) {
                by_source[source.index()] = tokens;
            }
        }
        Ok(by_source)
    }

    /// Today's hourly usage buckets (24 hours).
    pub fn today_hourly(&self) -> Result<Vec<HourlyUsage>> {
        let start = local_start_of_day().timestamp_millis() as f64 / 1000.0;

        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            r#"
            SELECT timestamp, tokens FROM usage_events
            WHERE timestamp >= ?
            "#,
        )?;

        let mut buckets = [0i64; 24];
        let rows = stmt.query_map(params![start], |row| {
            Ok((row.get::<_, f64>(0)?, row.get::<_, i64>(1)?))
        })?;

        for row in rows {
            let (ts, tokens) = row?;
            let Some(dt) =
                DateTime::<Utc>::from_timestamp(ts as i64, 0).map(|utc| utc.with_timezone(&Local))
            else {
                continue;
            };
            let hour = dt.hour() as usize;
            if hour < 24 {
                buckets[hour] += tokens;
            }
        }

        Ok(buckets
            .iter()
            .enumerate()
            .map(|(h, &t)| HourlyUsage {
                hour: h as u32,
                tokens: t,
            })
            .collect())
    }

    /// Daily totals for today and the six preceding local calendar days.
    pub fn last_seven_days(&self) -> Result<Vec<DailyUsage>> {
        let today = Local::now().date_naive();
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT COALESCE(SUM(tokens), 0) FROM usage_events WHERE timestamp >= ?1 AND timestamp < ?2",
        )?;
        let mut days = Vec::with_capacity(7);
        for age in (0..7).rev() {
            let date = today - Duration::days(age);
            let next = date + Duration::days(1);
            let local_start = |day: chrono::NaiveDate| {
                day.and_time(chrono::NaiveTime::MIN)
                    .and_local_timezone(Local)
                    .earliest()
                    .unwrap_or_else(Local::now)
                    .timestamp_millis() as f64
                    / 1000.0
            };
            let tokens = stmt.query_row(params![local_start(date), local_start(next)], |row| {
                row.get(0)
            })?;
            days.push(DailyUsage {
                date: date.format("%Y-%m-%d").to_string(),
                tokens,
            });
        }
        Ok(days)
    }

    /// Today's token breakdown (input / output / cache).
    pub fn today_breakdown(&self) -> Result<UsageBreakdown> {
        let start = local_start_of_day().timestamp_millis() as f64 / 1000.0;

        let db = self.db.lock().unwrap();
        let result: (i64, i64, i64, i64) = db
            .query_row(
                r#"
                SELECT
                    COALESCE(SUM(input_tokens), 0),
                    COALESCE(SUM(output_tokens), 0),
                    COALESCE(SUM(cache_read), 0),
                    COALESCE(SUM(cache_write), 0)
                FROM usage_events
                WHERE timestamp >= ?
                "#,
                params![start],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap_or((0, 0, 0, 0));

        Ok(UsageBreakdown {
            input: if result.0 > 0 { Some(result.0) } else { None },
            output: if result.1 > 0 { Some(result.1) } else { None },
            cache_read: if result.2 > 0 { Some(result.2) } else { None },
            cache_write: if result.3 > 0 { Some(result.3) } else { None },
        })
    }

    /// Recent events since a given time, oldest first.
    pub fn recent_events_since(
        &self,
        since: DateTime<Local>,
        limit: usize,
    ) -> Result<Vec<UsageEvent>> {
        let since_ts = since.timestamp_millis() as f64 / 1000.0;
        let db = self.db.lock().unwrap();

        let mut stmt = db.prepare(
            r#"
            SELECT id, source, timestamp, tokens, input_tokens, output_tokens,
                   cache_read, cache_write, file_path, is_estimated
            FROM usage_events
            WHERE timestamp >= ?
            ORDER BY timestamp ASC
            LIMIT ?
            "#,
        )?;

        let rows = stmt.query_map(params![since_ts, limit as i64], usage_event_from_row)?;

        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    /// Most recent stored events, newest first.
    pub fn recent_events(&self, limit: usize) -> Result<Vec<UsageEvent>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare(
            r#"
            SELECT id, source, timestamp, tokens, input_tokens, output_tokens,
                   cache_read, cache_write, file_path, is_estimated
            FROM usage_events
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )?;
        let rows = stmt.query_map(params![limit as i64], usage_event_from_row)?;
        let mut events = Vec::new();
        for row in rows {
            events.push(row?);
        }
        Ok(events)
    }

    // ── File cursors ────────────────────────────────────────

    /// Get the saved read cursor for a log file.
    #[cfg(test)]
    pub fn file_cursor(&self, path: &str) -> (u64, Option<String>) {
        let (offset, partial, _) = self.file_cursor_state(path);
        (offset, partial)
    }

    pub fn file_cursor_state(&self, path: &str) -> (u64, Option<String>, Option<String>) {
        let db = self.db.lock().unwrap();
        db.query_row(
            "SELECT byte_offset, partial_line, file_identity FROM file_cursors WHERE path = ?;",
            params![path],
            |row| Ok((row.get::<_, i64>(0)? as u64, row.get(1)?, row.get(2)?)),
        )
        .optional()
        .unwrap_or(None)
        .unwrap_or((0, None, None))
    }

    /// Set the read cursor for a log file.
    #[cfg(test)]
    pub fn set_file_cursor(&self, path: &str, offset: u64, partial: Option<&str>) {
        self.set_file_cursor_state(path, offset, partial, None);
    }

    pub fn set_file_cursor_state(
        &self,
        path: &str,
        offset: u64,
        partial: Option<&str>,
        file_identity: Option<&str>,
    ) {
        let db = self.db.lock().unwrap();
        let now = Local::now().timestamp_millis() as f64 / 1000.0;
        db.execute(
            r#"
            INSERT INTO file_cursors(path, byte_offset, partial_line, file_identity, updated_at)
            VALUES(?, ?, ?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET
                byte_offset=excluded.byte_offset,
                partial_line=excluded.partial_line,
                file_identity=COALESCE(excluded.file_identity, file_cursors.file_identity),
                updated_at=excluded.updated_at
            "#,
            params![path, offset as i64, partial, file_identity, now],
        )
        .ok();
    }

    /// Clear all saved file cursors (forces full re-read on next scan).
    pub fn clear_file_cursors(&self) {
        let db = self.db.lock().unwrap();
        db.execute("DELETE FROM file_cursors;", []).ok();
    }

    // ── Meta key/value ──────────────────────────────────────

    pub fn meta(&self, key: &str) -> Option<String> {
        let db = self.db.lock().unwrap();
        db.query_row(
            "SELECT value FROM meta WHERE key = ?;",
            params![key],
            |row| row.get(0),
        )
        .optional()
        .unwrap_or(None)
    }

    pub fn set_meta(&self, key: &str, value: &str) {
        let db = self.db.lock().unwrap();
        db.execute(
            "INSERT INTO meta(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value;",
            params![key, value],
        )
        .ok();
    }

    // ── Purge helpers ───────────────────────────────────────

    /// Drop old Cursor bubble estimate events (v1 format).
    pub fn purge_cursor_bubble_v1(&self) {
        let db = self.db.lock().unwrap();
        db.execute(
            "DELETE FROM usage_events WHERE id LIKE 'cursor:bubble:%' AND id NOT LIKE 'cursor:bubble:v2:%';",
            [],
        )
        .ok();
        let mut cache = self.known_ids.lock().unwrap();
        cache.retain(|id| {
            !(id.starts_with("cursor:bubble:") && !id.starts_with("cursor:bubble:v2:"))
        });
    }

    /// Drop all Cursor local estimates when Dashboard API becomes authoritative.
    pub fn purge_cursor_local_estimates(&self) {
        let db = self.db.lock().unwrap();
        db.execute(
            "DELETE FROM usage_events WHERE id LIKE 'cursor:bubble:%';",
            [],
        )
        .ok();
        let mut cache = self.known_ids.lock().unwrap();
        cache.retain(|id| !id.starts_with("cursor:bubble:"));
    }
}

// ── helpers ────────────────────────────────────────────────────

fn usage_event_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<UsageEvent> {
    let ts: f64 = row.get(2)?;
    let dt = DateTime::<Utc>::from_timestamp(ts as i64, (ts.fract() * 1e9) as u32)
        .map(|utc| utc.with_timezone(&Local))
        .unwrap_or_else(Local::now);
    let src_str: String = row.get(1)?;
    let source = UsageSource::from_str(&src_str).unwrap_or(UsageSource::ClaudeCode);

    Ok(UsageEvent {
        id: row.get(0)?,
        source,
        timestamp: dt,
        tokens: row.get(3)?,
        breakdown: UsageBreakdown {
            input: row.get(4)?,
            output: row.get(5)?,
            cache_read: row.get(6)?,
            cache_write: row.get(7)?,
        },
        file_path: row.get(8)?,
        is_estimated: row.get::<_, i32>(9)? != 0,
    })
}

impl UsageSource {
    fn as_ref_str(self) -> &'static str {
        match self {
            UsageSource::ClaudeCode => "claude_code",
            UsageSource::Codex => "codex",
            UsageSource::Cursor => "cursor",
            UsageSource::Grok => "grok",
            UsageSource::Pi => "pi",
            UsageSource::Amp => "amp",
            UsageSource::OpenCode => "opencode",
        }
    }

    fn from_str(s: &str) -> Option<UsageSource> {
        match s {
            "claude_code" => Some(UsageSource::ClaudeCode),
            "codex" => Some(UsageSource::Codex),
            "cursor" => Some(UsageSource::Cursor),
            "grok" => Some(UsageSource::Grok),
            "pi" => Some(UsageSource::Pi),
            "amp" => Some(UsageSource::Amp),
            "opencode" => Some(UsageSource::OpenCode),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event(id: &str, source: UsageSource, tokens: i64) -> UsageEvent {
        UsageEvent {
            id: id.to_string(),
            source,
            timestamp: Local::now(),
            tokens,
            breakdown: UsageBreakdown {
                input: Some(tokens / 2),
                output: Some(tokens / 2),
                cache_read: None,
                cache_write: None,
            },
            file_path: "/test/log.jsonl".to_string(),
            is_estimated: false,
        }
    }

    #[test]
    fn insert_and_query() {
        let store = UsageStore::open_in_memory().unwrap();
        let e = sample_event("test-1", UsageSource::ClaudeCode, 1000);
        assert!(store.insert_event(&e).unwrap());
        assert!(!store.insert_event(&e).unwrap()); // duplicate

        let (total, _by_source) = store.today_totals().unwrap();
        assert_eq!(total, 1000);
    }

    #[test]
    fn breakdown_totals() {
        let store = UsageStore::open_in_memory().unwrap();
        let e = sample_event("test-bd", UsageSource::Codex, 2000);
        store.insert_event(&e).unwrap();
        let bd = store.today_breakdown().unwrap();
        assert_eq!(bd.input, Some(1000));
        assert_eq!(bd.output, Some(1000));
    }

    #[test]
    fn meta_get_set() {
        let store = UsageStore::open_in_memory().unwrap();
        assert!(store.meta("foo").is_none());
        store.set_meta("foo", "bar");
        assert_eq!(store.meta("foo").as_deref(), Some("bar"));
    }

    #[test]
    fn file_cursor_roundtrip() {
        let store = UsageStore::open_in_memory().unwrap();
        assert_eq!(store.file_cursor("/tmp/x.log"), (0, None));
        store.set_file_cursor("/tmp/x.log", 1234, Some("partial"));
        let (off, partial) = store.file_cursor("/tmp/x.log");
        assert_eq!(off, 1234);
        assert_eq!(partial.as_deref(), Some("partial"));
    }

    #[test]
    fn reconstructs_cumulative_snapshot_from_delta_rows() {
        let store = UsageStore::open_in_memory().unwrap();
        let first = sample_event("codex:response-1", UsageSource::Codex, 100);
        let mut second = sample_event("codex:response-1#150", UsageSource::Codex, 50);
        second.breakdown.input = Some(20);
        second.breakdown.output = Some(30);
        store.insert_event(&first).unwrap();
        store.insert_event(&second).unwrap();

        let (tokens, breakdown) = store.cumulative_event_totals("codex:response-1").unwrap();
        assert_eq!(tokens, 150);
        assert_eq!(breakdown.input, Some(70));
        assert_eq!(breakdown.output, Some(80));
    }
}
