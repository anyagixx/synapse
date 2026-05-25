// MODULE_CONTRACT
// MODULE_ID: M-TRACKING
// PURPOSE: SQLite tracking and provenance ledger — records route-aware token usage plus autonomous run events by canonical project identity and provides stats
// SCOPE: Tracker struct, canonical project identity, explicit test state overrides, SQLite schema, route-aware token recording, provenance event recording, RTK coverage counts, session/adapter stats querying, route adoption and missed-route candidate querying, TrackingStats and RunEvent models
// DEPENDS: M-CONFIG
// LINKS:
//   → M-PROXY-ROUTER (depends) - adapter and route metadata source
//   → UC-002 (implements) - persisted execution evidence
//   → NFR-003 (traces_to) - token economy statistics and session analytics

// START_MODULE_MAP
// Tracker — Token usage and provenance ledger backed by SQLite
// Tracker::new_for_test — Test-only tracker with explicit db path, project key, and session id
// TrackingStats — Aggregate token economy statistics
// TrackingAdapterStat — Aggregate savings grouped by routed adapter
// TrackingSessionStat — Aggregate savings grouped by session id
// TrackingAdoptionStats — Aggregate RTK route adoption statistics
// TrackingMissedRouteStat — Passthrough command candidate for RTK routing review
// RunEvent — Autonomous run lifecycle event record
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v3.4.0 — Added cached SQLite connection with WAL and busy timeout]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;

const DEFAULT_MISSED_ROUTE_LIMIT: usize = 12;
const MAX_MISSED_ROUTE_COMMAND_CHARS: i64 = 160;

// START_public_api

// START_Tracker
pub struct Tracker {
    config: Config,
    conn: Mutex<Option<rusqlite::Connection>>,
    #[cfg(test)]
    db_path_override: Option<PathBuf>,
    #[cfg(test)]
    project_key_override: Option<String>,
    #[cfg(test)]
    session_id_override: Option<String>,
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
            conn: Mutex::new(None),
            #[cfg(test)]
            db_path_override: None,
            #[cfg(test)]
            project_key_override: None,
            #[cfg(test)]
            session_id_override: None,
        }
    }
    // END_tracker_new

    // START_CONTRACT_Tracker::new_for_test
    // PURPOSE: Create a test tracker with explicit data, project, and session state without mutating process globals
    // INPUTS: { config: &Config }, { data_home: &Path }, { project_root: &Path }, { session_id: Option<&str> }
    // OUTPUTS: { Self }
    // START_tracker_new_for_test
    #[cfg(test)]
    pub fn new_for_test(
        config: &Config,
        data_home: &Path,
        project_root: &Path,
        session_id: Option<&str>,
    ) -> Self {
        let project_key = project_root
            .canonicalize()
            .unwrap_or_else(|_| project_root.to_path_buf())
            .to_string_lossy()
            .to_string();
        Self {
            config: config.clone(),
            conn: Mutex::new(None),
            db_path_override: Some(data_home.join("synapse").join("tracking.db")),
            project_key_override: Some(project_key),
            session_id_override: session_id.map(ToOwned::to_owned),
        }
    }
    // END_tracker_new_for_test

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

    fn db_path_for(&self) -> anyhow::Result<PathBuf> {
        #[cfg(test)]
        if let Some(path) = &self.db_path_override {
            return Ok(path.clone());
        }
        Self::db_path()
    }

    fn project_key(&self) -> String {
        #[cfg(test)]
        if let Some(project) = &self.project_key_override {
            return project.clone();
        }
        current_project_key()
    }

    fn session_id(&self) -> String {
        #[cfg(test)]
        if let Some(session_id) = &self.session_id_override {
            return session_id.clone();
        }
        current_session_id()
    }

    // START_CONTRACT_Tracker::with_conn
    // PURPOSE: Run one SQLite operation through the cached tracker connection
    // INPUTS: { operation: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T> }
    // OUTPUTS: { anyhow::Result<T> }
    // SIDE_EFFECTS: opens and initializes tracking DB on first use
    // START_tracker_with_conn
    fn with_conn<T>(
        &self,
        operation: impl FnOnce(&rusqlite::Connection) -> rusqlite::Result<T>,
    ) -> anyhow::Result<T> {
        let mut guard = self
            .conn
            .lock()
            .map_err(|_| anyhow::anyhow!("tracker connection lock poisoned"))?;
        if guard.is_none() {
            let db_path = self.db_path_for()?;
            *guard = Some(open_tracking_connection(&db_path)?);
        }
        let conn = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("tracker connection unavailable after open"))?;
        operation(conn).map_err(|e| anyhow::anyhow!("tracking DB operation failed: {}", e))
    }
    // END_tracker_with_conn

    #[cfg(test)]
    fn cached_connection_is_initialized_for_test(&self) -> bool {
        self.conn
            .lock()
            .map(|guard| guard.is_some())
            .unwrap_or(false)
    }

    // START_CONTRACT_Tracker::record
    // PURPOSE: Record a proxied command with token counts into SQLite
    // INPUTS: { cmd: &str — command string }, { input_tokens: u32 }, { output_tokens: u32 }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to SQLite database
    // LINKS:
    //   → NFR-003 (traces_to) - token usage is persisted for savings reporting
    // START_tracker_record
    pub async fn record(
        &self,
        cmd: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> anyhow::Result<()> {
        self.record_routed(cmd, input_tokens, output_tokens, "passthrough", "")
            .await
    }
    // END_tracker_record

    // START_CONTRACT_Tracker::record_routed
    // PURPOSE: Record a proxied command with adapter, route, and session metadata
    // INPUTS: { cmd: &str }, { input_tokens: u32 }, { output_tokens: u32 }, { adapter: &str }, { route_key: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to SQLite database
    // LINKS:
    //   → M-PROXY-ROUTER (depends) - stores adapter route decisions
    //   → UC-002 (implements) - route-aware command records become replayable execution evidence
    //   → NFR-003 (traces_to) - session and adapter analytics quantify token savings
    // START_tracker_record_routed
    pub async fn record_routed(
        &self,
        cmd: &str,
        input_tokens: u32,
        output_tokens: u32,
        adapter: &str,
        route_key: &str,
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
        let project = self.project_key();
        let session_id = self.session_id();

        self.with_conn(|conn| {
            conn.execute(
            "INSERT INTO commands (original_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, project_path, adapter, route_key, session_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                rusqlite::params![
                    cmd,
                    input_tokens,
                    output_tokens,
                    saved,
                    savings_pct,
                    project,
                    adapter,
                    route_key,
                    session_id
                ],
            )
            .map(|_| ())
        })
    }
    // END_tracker_record_routed

    // START_CONTRACT_Tracker::record_run_event
    // PURPOSE: Record an autonomous run lifecycle event into SQLite provenance ledger
    // INPUTS: { run_id: &str }, { event_type: &str }, { module_id: &str }, { phase: &str }, { status: &str }, { detail: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to SQLite database
    // LINKS:
    //   → UC-002 (implements) - run events persist bounded workflow provenance
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
        let project = self.project_key();
        self.with_conn(|conn| {
            conn.execute(
            "INSERT INTO run_events (run_id, event_type, module_id, phase, status, detail, project_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![run_id, event_type, module_id, phase, status, detail, project],
            )
            .map(|_| ())
        })
    }
    // END_tracker_record_run_event

    // START_CONTRACT_Tracker::get_run_events
    // PURPOSE: Retrieve autonomous run lifecycle events for the current project
    // INPUTS: { run_id: Option<&str> }
    // OUTPUTS: { anyhow::Result<Vec<RunEvent>> }
    // LINKS:
    //   → UC-002 (implements) - run event retrieval supports audit and replay
    // START_tracker_get_run_events
    pub async fn get_run_events(&self, run_id: Option<&str>) -> anyhow::Result<Vec<RunEvent>> {
        let project = self.project_key();
        self.with_conn(|conn| {
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
        })
    }
    // END_tracker_get_run_events
    // START_CONTRACT_Tracker::get_stats
    // PURPOSE: Retrieve aggregate token economy stats for the current project
    // OUTPUTS: { anyhow::Result<TrackingStats> }
    // LINKS:
    //   → NFR-003 (traces_to) - reports command, adapter, and session savings
    // START_tracker_get_stats
    pub async fn get_stats(&self) -> anyhow::Result<TrackingStats> {
        let project = self.project_key();
        self.with_conn(|conn| {
            let mut stats = TrackingStats::default();

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
                "SELECT
                COUNT(DISTINCT CASE
                    WHEN adapter IS NOT NULL AND adapter != '' THEN adapter
                    ELSE 'passthrough'
                END),
                COUNT(DISTINCT CASE
                    WHEN session_id IS NOT NULL AND session_id != '' THEN session_id
                    ELSE 'local'
                END)
             FROM commands
             WHERE project_path = ?1",
            )?;
            stmt.query_row(rusqlite::params![project.clone()], |row| {
                stats.adapter_groups = row.get(0)?;
                stats.session_groups = row.get(1)?;
                Ok(())
            })?;

            let mut stmt = conn.prepare(
                "SELECT CASE
                    WHEN route_key IS NOT NULL AND route_key != '' THEN route_key
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
             LIMIT 12",
            )?;
            let rows = stmt.query_map(rusqlite::params![project.clone()], |row| {
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

            let mut stmt = conn.prepare(
                "SELECT CASE
                    WHEN adapter IS NOT NULL AND adapter != '' THEN adapter
                    ELSE 'passthrough'
                END AS adapter_name,
                COUNT(*) as count,
                COALESCE(SUM(saved_tokens),0) as saved,
                COALESCE(AVG(savings_pct),0) as avg_pct
             FROM commands
             WHERE project_path = ?1
             GROUP BY adapter_name
             ORDER BY saved DESC, count DESC
             LIMIT 12",
            )?;
            let rows = stmt.query_map(rusqlite::params![project.clone()], |row| {
                Ok(TrackingAdapterStat {
                    adapter: row.get(0)?,
                    count: row.get(1)?,
                    saved_tokens: row.get(2)?,
                    avg_savings_pct: row.get(3)?,
                })
            })?;
            for row in rows {
                stats.top_adapters.push(row?);
            }

            let mut stmt = conn.prepare(
                "SELECT CASE
                    WHEN session_id IS NOT NULL AND session_id != '' THEN session_id
                    ELSE 'local'
                END AS session_name,
                COUNT(*) as count,
                COALESCE(SUM(input_tokens),0) as input_total,
                COALESCE(SUM(output_tokens),0) as output_total,
                COALESCE(SUM(saved_tokens),0) as saved,
                COALESCE(AVG(savings_pct),0) as avg_pct,
                MAX(timestamp) as last_seen
             FROM commands
             WHERE project_path = ?1
             GROUP BY session_name
             ORDER BY last_seen DESC
             LIMIT 12",
            )?;
            let rows = stmt.query_map(rusqlite::params![project.clone()], |row| {
                Ok(TrackingSessionStat {
                    session_id: row.get(0)?,
                    count: row.get(1)?,
                    input_tokens: row.get(2)?,
                    output_tokens: row.get(3)?,
                    saved_tokens: row.get(4)?,
                    avg_savings_pct: row.get(5)?,
                    last_seen: row.get(6)?,
                })
            })?;
            for row in rows {
                stats.recent_sessions.push(row?);
            }
            stats.adoption = query_adoption_stats(conn, &project, DEFAULT_MISSED_ROUTE_LIMIT)?;
            Ok(stats)
        })
    }
    // END_tracker_get_stats

    // START_CONTRACT_Tracker::get_adoption_stats
    // PURPOSE: Retrieve RTK route adoption and missed-route candidates for the current project
    // INPUTS: { limit: usize }
    // OUTPUTS: { anyhow::Result<TrackingAdoptionStats> }
    // LINKS:
    //   → NFR-003 (traces_to) - measured adoption guides token-saving rollout
    //   → Phase-56 (implements) - tracking-backed missed-route surface
    // START_tracker_get_adoption_stats
    pub async fn get_adoption_stats(&self, limit: usize) -> anyhow::Result<TrackingAdoptionStats> {
        let project = self.project_key();
        self.with_conn(|conn| query_adoption_stats(conn, &project, limit))
    }
    // END_tracker_get_adoption_stats
}

// START_CONTRACT_open_tracking_connection
// PURPOSE: Open and initialize the SQLite tracking connection once per Tracker
// INPUTS: { db_path: &Path }
// OUTPUTS: { anyhow::Result<rusqlite::Connection> }
// SIDE_EFFECTS: creates parent directory, enables WAL/busy_timeout, migrates schema
// START_open_tracking_connection
fn open_tracking_connection(db_path: &Path) -> anyhow::Result<rusqlite::Connection> {
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| anyhow::anyhow!("open tracking DB {}: {}", db_path.display(), e))?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA busy_timeout=5000;")?;
    ensure_schema(&conn)?;
    Ok(conn)
}
// END_open_tracking_connection

// START_CONTRACT_query_adoption_stats
// PURPOSE: Query route adoption counts and passthrough commands that may be missed RTK routes
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }, { limit: usize }
// OUTPUTS: { rusqlite::Result<TrackingAdoptionStats> }
// LINKS:
//   → M-TRACKING (depends) - local tracking database query surface
//   → Phase-56 (implements) - measured RTK adoption and missed-route reporting
// START_query_adoption_stats
fn query_adoption_stats(
    conn: &rusqlite::Connection,
    project: &str,
    limit: usize,
) -> rusqlite::Result<TrackingAdoptionStats> {
    let normalized_limit = if limit == 0 {
        DEFAULT_MISSED_ROUTE_LIMIT
    } else {
        limit
    };
    let mut adoption = TrackingAdoptionStats::default();
    let mut stmt = conn.prepare(
        "SELECT
            COUNT(*),
            COALESCE(SUM(CASE
                WHEN route_key IS NOT NULL
                    AND route_key != ''
                    AND adapter IS NOT NULL
                    AND adapter != ''
                    AND adapter != 'passthrough'
                THEN 1 ELSE 0 END),0),
            COALESCE(SUM(CASE
                WHEN route_key IS NULL
                    OR route_key = ''
                    OR adapter IS NULL
                    OR adapter = ''
                    OR adapter = 'passthrough'
                THEN 1 ELSE 0 END),0)
         FROM commands
         WHERE project_path = ?1",
    )?;
    stmt.query_row(rusqlite::params![project], |row| {
        adoption.total_commands = row.get(0)?;
        adoption.routed_commands = row.get(1)?;
        adoption.passthrough_commands = row.get(2)?;
        Ok(())
    })?;
    adoption.route_adoption_pct = if adoption.total_commands == 0 {
        0.0
    } else {
        adoption.routed_commands as f64 / adoption.total_commands as f64 * 100.0
    };

    let mut stmt = conn.prepare(
        "SELECT substr(original_cmd, 1, ?3), COUNT(*) as count, MAX(timestamp) as last_seen
         FROM commands
         WHERE project_path = ?1
            AND (route_key IS NULL
                OR route_key = ''
                OR adapter IS NULL
                OR adapter = ''
                OR adapter = 'passthrough')
            AND original_cmd NOT LIKE 'syn %'
            AND original_cmd NOT LIKE 'rtk %'
         GROUP BY original_cmd
         ORDER BY count DESC, last_seen DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![
            project,
            normalized_limit as i64,
            MAX_MISSED_ROUTE_COMMAND_CHARS
        ],
        |row| {
            Ok(TrackingMissedRouteStat {
                command: row.get(0)?,
                count: row.get(1)?,
                last_seen: row.get(2)?,
            })
        },
    )?;
    for row in rows {
        adoption.missed_route_candidates.push(row?);
    }
    Ok(adoption)
}
// END_query_adoption_stats

// START_CONTRACT_current_project_key
// PURPOSE: Return a stable canonical project identity for per-project tracking isolation
// OUTPUTS: { String }
// LINKS:
//   → NFR-002 (traces_to) - canonical project paths prevent cross-project tracking bleed
// START_current_project_key
fn current_project_key() -> String {
    std::env::current_dir()
        .ok()
        .map(|p| p.canonicalize().unwrap_or(p))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default()
}
// END_current_project_key

// START_CONTRACT_current_session_id
// PURPOSE: Return the current token-economy session identity from agent or Synapse environment
// OUTPUTS: { String }
// LINKS:
//   → NFR-003 (traces_to) - session identity enables per-agent-run economics reporting
// START_current_session_id
fn current_session_id() -> String {
    [
        "SYNAPSE_SESSION_ID",
        "OPENCODE_SESSION_ID",
        "CLAUDECODE_SESSION_ID",
    ]
    .iter()
    .find_map(|key| std::env::var(key).ok())
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
    .unwrap_or_else(|| "local".into())
}
// END_current_session_id

// START_CONTRACT_ensure_schema
// PURPOSE: Ensure the SQLite tracking schema exists before reads or writes
// INPUTS: { conn: &rusqlite::Connection }
// OUTPUTS: { rusqlite::Result<()> }
// SIDE_EFFECTS: creates commands table when absent
// LINKS:
//   → NFR-002 (traces_to) - schema creation and migration errors propagate to callers
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
    )?;
    ensure_command_column(conn, "adapter", "TEXT DEFAULT ''")?;
    ensure_command_column(conn, "route_key", "TEXT DEFAULT ''")?;
    ensure_command_column(conn, "session_id", "TEXT DEFAULT ''")?;
    Ok(())
}
// END_ensure_schema

// START_CONTRACT_ensure_command_column
// PURPOSE: Add one commands-table column when opening an older tracking database
// INPUTS: { conn: &rusqlite::Connection }, { name: &str }, { definition: &str }
// OUTPUTS: { rusqlite::Result<()> }
// SIDE_EFFECTS: may alter the commands table
// LINKS:
//   → NFR-002 (traces_to) - older tracking databases migrate without data loss
// START_ensure_command_column
fn ensure_command_column(
    conn: &rusqlite::Connection,
    name: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    if command_column_exists(conn, name)? {
        return Ok(());
    }
    let sql = format!("ALTER TABLE commands ADD COLUMN {} {}", name, definition);
    conn.execute(&sql, [])?;
    Ok(())
}
// END_ensure_command_column

// START_CONTRACT_command_column_exists
// PURPOSE: Check whether the commands table already has one column
// INPUTS: { conn: &rusqlite::Connection }, { name: &str }
// OUTPUTS: { rusqlite::Result<bool> }
// LINKS:
//   → NFR-002 (traces_to) - schema migration is idempotent
// START_command_column_exists
fn command_column_exists(conn: &rusqlite::Connection, name: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(commands)")?;
    let columns = stmt.query_map([], |row| row.get::<_, String>(1))?;
    for column in columns {
        if column? == name {
            return Ok(true);
        }
    }
    Ok(false)
}
// END_command_column_exists

// START_TrackingCommandStat
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingCommandStat {
    pub command: String,
    pub count: u64,
    pub saved_tokens: u64,
    pub avg_savings_pct: f64,
}
// END_TrackingCommandStat

// START_TrackingAdapterStat
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingAdapterStat {
    pub adapter: String,
    pub count: u64,
    pub saved_tokens: u64,
    pub avg_savings_pct: f64,
}
// END_TrackingAdapterStat

// START_TrackingSessionStat
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingSessionStat {
    pub session_id: String,
    pub count: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub saved_tokens: u64,
    pub avg_savings_pct: f64,
    pub last_seen: String,
}
// END_TrackingSessionStat

// START_TrackingMissedRouteStat
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingMissedRouteStat {
    pub command: String,
    pub count: u64,
    pub last_seen: String,
}
// END_TrackingMissedRouteStat

// START_TrackingAdoptionStats
#[derive(serde::Serialize, Default, Debug, Clone)]
pub struct TrackingAdoptionStats {
    pub total_commands: u64,
    pub routed_commands: u64,
    pub passthrough_commands: u64,
    pub route_adoption_pct: f64,
    pub missed_route_candidates: Vec<TrackingMissedRouteStat>,
}
// END_TrackingAdoptionStats

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
    pub adapter_groups: u64,
    pub session_groups: u64,
    pub top_commands: Vec<TrackingCommandStat>,
    pub top_adapters: Vec<TrackingAdapterStat>,
    pub recent_sessions: Vec<TrackingSessionStat>,
    pub adoption: TrackingAdoptionStats,
}
// END_TrackingStats

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracker_records_and_reads_run_events() {
        let data_home = tempfile::tempdir().unwrap();
        let project_home = tempfile::tempdir().unwrap();
        let tracker = Tracker::new_for_test(
            &Config::default(),
            data_home.path(),
            project_home.path(),
            None,
        );
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
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "create_run");
        assert_eq!(events[1].status, "running");
    }

    #[tokio::test]
    async fn tracker_records_routed_adapter_and_session_stats() {
        let data_home = tempfile::tempdir().unwrap();
        let project_home = tempfile::tempdir().unwrap();
        let tracker = Tracker::new_for_test(
            &Config::default(),
            data_home.path(),
            project_home.path(),
            Some("session-router-test"),
        );
        tracker
            .record_routed("cargo test --all", 100, 25, "rust-cargo", "cargo test")
            .await
            .unwrap();
        tracker
            .record_routed("docker ps", 80, 20, "infra-cli", "docker ps")
            .await
            .unwrap();
        tracker.record("git status", 40, 35).await.unwrap();

        let adoption = tracker.get_adoption_stats(10).await.unwrap();
        let stats = tracker.get_stats().await.unwrap();

        assert_eq!(stats.total_commands, 3);
        assert_eq!(stats.adapter_groups, 3);
        assert_eq!(stats.session_groups, 1);
        assert_eq!(stats.top_commands[0].command, "cargo test");
        assert_eq!(stats.top_adapters[0].adapter, "rust-cargo");
        assert_eq!(stats.recent_sessions[0].session_id, "session-router-test");
        assert_eq!(stats.recent_sessions[0].saved_tokens, 140);
        assert_eq!(adoption.total_commands, 3);
        assert_eq!(adoption.routed_commands, 2);
        assert_eq!(adoption.passthrough_commands, 1);
        assert_eq!(adoption.missed_route_candidates[0].command, "git status");
    }

    #[tokio::test]
    async fn tracker_reuses_cached_connection_after_first_operation() {
        let data_home = tempfile::tempdir().unwrap();
        let project_home = tempfile::tempdir().unwrap();
        let tracker = Tracker::new_for_test(
            &Config::default(),
            data_home.path(),
            project_home.path(),
            Some("session-cache-test"),
        );

        assert!(!tracker.cached_connection_is_initialized_for_test());
        tracker
            .record_routed("cargo check", 100, 40, "rust-cargo", "cargo check")
            .await
            .unwrap();
        assert!(tracker.cached_connection_is_initialized_for_test());

        let stats = tracker.get_stats().await.unwrap();
        assert_eq!(stats.total_commands, 1);
        assert!(tracker.cached_connection_is_initialized_for_test());
    }
}

// END_public_api
