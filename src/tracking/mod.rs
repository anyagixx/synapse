// MODULE_CONTRACT
// MODULE_ID: M-TRACKING
// PURPOSE: SQLite tracking and provenance ledger — records token usage plus autonomous run events by canonical project identity and provides stats
// SCOPE: Tracker struct, canonical project identity, SQLite schema, token recording, provenance event recording, stats querying, TrackingStats and RunEvent models
// DEPENDS: M-CONFIG
// LINKS: tracking.db

// START_MODULE_MAP
// Tracker — Token usage and provenance ledger backed by SQLite
// TrackingStats — Aggregate token economy statistics
// RunEvent — Autonomous run lifecycle event record
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.9.0 — Isolated run-event unit test against parallel current-directory mutation]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::path::PathBuf;

// START_public_api

// START_Tracker
pub struct Tracker {
    config: Config,
}

// END_Tracker

impl Tracker {
    // START_CONTRACT_Tracker::new
    // PURPOSE: Create a new Tracker with the given config
    // INPUTS: { config: &Config }
    // OUTPUTS: { Self }
    // START_tracker_new
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }
    // END_tracker_new

    // START_CONTRACT_Tracker::db_path
    // PURPOSE: Return the path to the SQLite tracking database
    // OUTPUTS: { anyhow::Result<PathBuf> }
    // START_tracker_db_path
    pub fn db_path() -> anyhow::Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find data directory"))?
            .join("synapse");
        Ok(data_dir.join("tracking.db"))
    }
    // END_tracker_db_path

    #[allow(dead_code)]
    fn project_db_path() -> anyhow::Result<PathBuf> {
        let project_hash = std::env::current_dir()
            .ok()
            .map(|p| {
                use std::hash::{Hash, Hasher};
                let mut h = std::collections::hash_map::DefaultHasher::new();
                p.to_string_lossy().hash(&mut h);
                format!("{:x}", h.finish())
            })
            .unwrap_or_default();
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find data directory"))?
            .join("synapse");
        Ok(data_dir.join(format!("tracking-{}.db", project_hash)))
    }

    // START_CONTRACT_Tracker::record
    // PURPOSE: Record a proxied command with token counts into SQLite
    // INPUTS: { cmd: &str — command string }, { input_tokens: u32 }, { output_tokens: u32 }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to SQLite database
    // START_tracker_record
    pub async fn record(
        &self,
        cmd: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> anyhow::Result<()> {
        if !self.config.tracking.enabled() {
            return Ok(());
        }
        let saved = input_tokens.saturating_sub(output_tokens);
        let savings_pct = if input_tokens > 0 {
            (saved as f64 / input_tokens as f64 * 100.0) as u32
        } else {
            0
        };
        let project = current_project_key();

        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| anyhow::anyhow!("open tracking DB {}: {}", db_path.display(), e))?;
        ensure_schema(&conn)?;
        conn.execute(
            "INSERT INTO commands (original_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, project_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![cmd, input_tokens, output_tokens, saved, savings_pct, project],
        )?;
        Ok(())
    }
    // END_tracker_record

    // START_CONTRACT_Tracker::record_run_event
    // PURPOSE: Record an autonomous run lifecycle event into SQLite provenance ledger
    // INPUTS: { run_id: &str }, { event_type: &str }, { module_id: &str }, { phase: &str }, { status: &str }, { detail: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to SQLite database
    // START_tracker_record_run_event
    pub async fn record_run_event(
        &self,
        run_id: &str,
        event_type: &str,
        module_id: &str,
        phase: &str,
        status: &str,
        detail: &str,
    ) -> anyhow::Result<()> {
        if !self.config.tracking.enabled() {
            return Ok(());
        }
        let project = current_project_key();
        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| anyhow::anyhow!("open tracking DB {}: {}", db_path.display(), e))?;
        ensure_schema(&conn)?;
        conn.execute(
            "INSERT INTO run_events (run_id, event_type, module_id, phase, status, detail, project_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![run_id, event_type, module_id, phase, status, detail, project],
        )?;
        Ok(())
    }
    // END_tracker_record_run_event

    // START_CONTRACT_Tracker::get_run_events
    // PURPOSE: Retrieve autonomous run lifecycle events for the current project
    // INPUTS: { run_id: Option<&str> }
    // OUTPUTS: { anyhow::Result<Vec<RunEvent>> }
    // START_tracker_get_run_events
    pub async fn get_run_events(&self, run_id: Option<&str>) -> anyhow::Result<Vec<RunEvent>> {
        let db_path = Self::db_path()?;
        let project = current_project_key();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| anyhow::anyhow!("open tracking DB {}: {}", db_path.display(), e))?;
        ensure_schema(&conn)?;
        let mut events = Vec::new();
        match run_id {
            Some(run_id) => {
                let mut stmt = conn.prepare(
                    "SELECT run_id, event_type, module_id, phase, status, detail, timestamp
                     FROM run_events
                     WHERE project_path = ?1 AND run_id = ?2
                     ORDER BY id ASC",
                )?;
                let rows = stmt.query_map(rusqlite::params![project, run_id], |row| {
                    Ok(RunEvent {
                        run_id: row.get(0)?,
                        event_type: row.get(1)?,
                        module_id: row.get(2)?,
                        phase: row.get(3)?,
                        status: row.get(4)?,
                        detail: row.get(5)?,
                        timestamp: row.get(6)?,
                    })
                })?;
                for row in rows {
                    events.push(row?);
                }
            }
            None => {
                let mut stmt = conn.prepare(
                    "SELECT run_id, event_type, module_id, phase, status, detail, timestamp
                     FROM run_events
                     WHERE project_path = ?1
                     ORDER BY id ASC",
                )?;
                let rows = stmt.query_map(rusqlite::params![project], |row| {
                    Ok(RunEvent {
                        run_id: row.get(0)?,
                        event_type: row.get(1)?,
                        module_id: row.get(2)?,
                        phase: row.get(3)?,
                        status: row.get(4)?,
                        detail: row.get(5)?,
                        timestamp: row.get(6)?,
                    })
                })?;
                for row in rows {
                    events.push(row?);
                }
            }
        }
        Ok(events)
    }
    // END_tracker_get_run_events
    pub async fn get_stats(&self) -> anyhow::Result<TrackingStats> {
        let db_path = Self::db_path()?;
        let project = current_project_key();
        let mut stats = TrackingStats::default();

        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| anyhow::anyhow!("open tracking DB {}: {}", db_path.display(), e))?;
        ensure_schema(&conn)?;

        let mut stmt = conn.prepare(
            "SELECT COUNT(*), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0),
                    COALESCE(SUM(saved_tokens),0), COALESCE(AVG(savings_pct),0)
             FROM commands WHERE project_path = ?1",
        )?;
        stmt.query_row(rusqlite::params![project.clone()], |row| {
            stats.total_commands = row.get(0)?;
            stats.total_input_tokens = row.get(1)?;
            stats.total_output_tokens = row.get(2)?;
            stats.total_saved_tokens = row.get(3)?;
            stats.avg_savings_pct = row.get(4)?;
            Ok(())
        })?;

        let mut stmt = conn.prepare(
            "SELECT CASE
                    WHEN instr(original_cmd, ' ') > 0 THEN substr(original_cmd, 1, instr(original_cmd, ' ') - 1)
                    ELSE original_cmd
                END AS command_name,
                COUNT(*) as count,
                COALESCE(SUM(saved_tokens),0) as saved,
                COALESCE(AVG(savings_pct),0) as avg_pct
             FROM commands
             WHERE project_path = ?1
             GROUP BY command_name
             ORDER BY saved DESC, count DESC
             LIMIT 5",
        )?;
        let rows = stmt.query_map(rusqlite::params![project], |row| {
            Ok(TrackingCommandStat {
                command: row.get(0)?,
                count: row.get(1)?,
                saved_tokens: row.get(2)?,
                avg_savings_pct: row.get(3)?,
            })
        })?;
        for row in rows {
            stats.top_commands.push(row?);
        }
        Ok(stats)
    }
    // END_tracker_get_stats
}

// START_CONTRACT_current_project_key
// PURPOSE: Return a stable canonical project identity for per-project tracking isolation
// OUTPUTS: { String }
// START_current_project_key
fn current_project_key() -> String {
    std::env::current_dir()
        .ok()
        .map(|p| p.canonicalize().unwrap_or(p))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}
// END_current_project_key

// START_CONTRACT_ensure_schema
// PURPOSE: Ensure the SQLite tracking schema exists before reads or writes
// INPUTS: { conn: &rusqlite::Connection }
// OUTPUTS: { rusqlite::Result<()> }
// SIDE_EFFECTS: creates commands table when absent
// START_ensure_schema
fn ensure_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS commands (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            original_cmd TEXT NOT NULL,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            saved_tokens INTEGER NOT NULL,
            savings_pct REAL NOT NULL,
            project_path TEXT DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS run_events (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            run_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            module_id TEXT NOT NULL,
            phase TEXT NOT NULL,
            status TEXT NOT NULL,
            detail TEXT NOT NULL,
            project_path TEXT DEFAULT ''
        );",
    )
}
// END_ensure_schema

// START_TrackingCommandStat
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingCommandStat {
    pub command: String,
    pub count: u64,
    pub saved_tokens: u64,
    pub avg_savings_pct: f64,
}
// END_TrackingCommandStat

// START_RunEvent
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct RunEvent {
    pub run_id: String,
    pub event_type: String,
    pub module_id: String,
    pub phase: String,
    pub status: String,
    pub detail: String,
    pub timestamp: String,
}
// END_RunEvent

// START_TrackingStats
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingStats {
    pub total_commands: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_saved_tokens: u64,
    pub avg_savings_pct: f64,
    pub top_commands: Vec<TrackingCommandStat>,
}
// END_TrackingStats

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracker_records_and_reads_run_events() {
        let _cwd = crate::utils::test_cwd_lock().lock().await;
        let data_home = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("XDG_DATA_HOME", data_home.path());
        }
        let project_home = tempfile::tempdir().unwrap();
        let old_cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
        std::env::set_current_dir(project_home.path()).unwrap();
        let tracker = Tracker::new(&Config::default());
        tracker
            .record_run_event(
                "run-test-1",
                "create_run",
                "M-RUNNER",
                "Phase-17",
                "ready",
                "created bounded run",
            )
            .await
            .unwrap();
        tracker
            .record_run_event(
                "run-test-1",
                "start_run",
                "M-RUNNER",
                "Phase-17",
                "running",
                "started bounded run",
            )
            .await
            .unwrap();

        let events = tracker.get_run_events(Some("run-test-1")).await.unwrap();
        std::env::set_current_dir(old_cwd).unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "create_run");
        assert_eq!(events[1].status, "running");
    }
}

// END_public_api
