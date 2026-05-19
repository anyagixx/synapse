// MODULE_CONTRACT
// MODULE_ID: M-TRACKING
// PURPOSE: SQLite token tracker — records command token usage by canonical project identity and provides stats
// SCOPE: Tracker struct, canonical project identity, SQLite schema, token recording, stats querying, TrackingStats model
// DEPENDS: M-CONFIG
// LINKS: tracking.db

// START_MODULE_MAP
// Tracker — Token usage tracker backed by SQLite
// TrackingStats — Aggregate token economy statistics
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Propagated SQLite errors and switched tracking identity to canonical project path]
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

    // START_CONTRACT_Tracker::get_stats
    // PURPOSE: Retrieve aggregate tracking statistics for the current project
    // OUTPUTS: { anyhow::Result<TrackingStats> }
    // START_tracker_get_stats
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
// END_public_api
