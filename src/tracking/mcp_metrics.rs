// MODULE_CONTRACT
// MODULE_ID: M-TRACKING-MCP-METRICS
// PURPOSE: SQLite-backed MCP runtime metrics for request counts, active request snapshots, latency summaries, tool errors, and last-seen timestamps
// SCOPE: MCP metrics schema, best-effort tool-call recording, aggregate stats queries, retention-aware cleanup, project identity reuse without token-savings analytics bloat
// DEPENDS: M-CONFIG, M-TRACKING
// LINKS:
//   -> M-TRACKING (depends) - SQLite connection and project identity conventions
//   -> M-CONFIG (depends) - observability toggles and retention bounds

// START_MODULE_MAP
// McpCallStatus — Normalized MCP call status values
// McpToolMetric — Per-tool aggregate metrics
// McpMetricsSnapshot — Runtime MCP metrics aggregate payload
// Tracker::start_mcp_request — Record an active MCP request
// Tracker::finish_mcp_request — Finish an active request and persist call metrics
// Tracker::record_mcp_call — Persist one completed MCP call metric
// Tracker::get_mcp_metrics — Query aggregate MCP runtime metrics
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 - Applied observability toggles and retention cleanup]
// END_CHANGE_SUMMARY

// START_public_api

// START_McpCallStatus
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpCallStatus {
    Ok,
    Error,
}
// END_McpCallStatus

impl McpCallStatus {
    // START_CONTRACT_McpCallStatus::as_str
    // PURPOSE: Return the persisted lowercase MCP call status
    // OUTPUTS: { &'static str }
    // LINKS:
    //   -> NFR-003 (traces_to) - runtime metrics quantify MCP tool reliability
    // START_mcp_call_status_as_str
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
        }
    }
    // END_mcp_call_status_as_str
}

// START_McpToolMetric
#[derive(serde::Serialize, Default, Debug, Clone, PartialEq)]
pub struct McpToolMetric {
    pub tool_name: String,
    pub calls: u64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: u64,
    pub errors: u64,
    pub last_seen: String,
}
// END_McpToolMetric

// START_McpMetricsSnapshot
#[derive(serde::Serialize, Default, Debug, Clone, PartialEq)]
pub struct McpMetricsSnapshot {
    pub total_calls: u64,
    pub active_requests: u64,
    pub error_calls: u64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: u64,
    pub last_seen: String,
    pub tools: Vec<McpToolMetric>,
}
// END_McpMetricsSnapshot

impl super::Tracker {
    // START_CONTRACT_Tracker::start_mcp_request
    // PURPOSE: Record an active MCP request and return its active row id
    // INPUTS: { tool_name: &str }
    // OUTPUTS: { anyhow::Result<Option<i64>> }
    // SIDE_EFFECTS: writes to mcp_active_requests when tracking is enabled
    // LINKS:
    //   -> NFR-003 (traces_to) - active request tracking exposes runtime MCP load
    // START_tracker_start_mcp_request
    pub async fn start_mcp_request(&self, tool_name: &str) -> anyhow::Result<Option<i64>> {
        if !self.config.tracking.enabled() || !self.config.observability.mcp_metrics_enabled() {
            return Ok(None);
        }
        let project = self.project_key();
        let tool_name = normalize_tool_name(tool_name);
        self.with_conn(|conn| {
            ensure_mcp_metrics_schema(conn)?;
            conn.execute(
                "INSERT INTO mcp_active_requests (tool_name, project_path) VALUES (?1, ?2)",
                rusqlite::params![tool_name, project],
            )?;
            Ok(Some(conn.last_insert_rowid()))
        })
    }
    // END_tracker_start_mcp_request

    // START_CONTRACT_Tracker::finish_mcp_request
    // PURPOSE: Remove an active MCP request and persist the completed call metrics
    // INPUTS: { active_id: Option<i64> }, { tool_name: &str }, { duration_ms: u64 }, { status: McpCallStatus }, { error_message: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to mcp_tool_calls and deletes from mcp_active_requests
    // LINKS:
    //   -> NFR-003 (traces_to) - completed MCP call metrics quantify latency and errors
    // START_tracker_finish_mcp_request
    pub async fn finish_mcp_request(
        &self,
        active_id: Option<i64>,
        tool_name: &str,
        duration_ms: u64,
        status: McpCallStatus,
        error_message: &str,
    ) -> anyhow::Result<()> {
        if !self.config.tracking.enabled() || !self.config.observability.mcp_metrics_enabled() {
            return Ok(());
        }
        let project = self.project_key();
        let tool_name = normalize_tool_name(tool_name);
        let duration_ms = u64_to_i64_saturating(duration_ms);
        let error_message = truncate_error_message(error_message);
        let retention_days = self.config.observability.mcp_metrics_retention_days();
        self.with_conn(|conn| {
            ensure_mcp_metrics_schema(conn)?;
            if let Some(active_id) = active_id {
                conn.execute(
                    "DELETE FROM mcp_active_requests WHERE id = ?1 AND project_path = ?2",
                    rusqlite::params![active_id, project],
                )?;
            }
            insert_mcp_call(
                conn,
                &tool_name,
                duration_ms,
                status,
                &error_message,
                &project,
            )?;
            prune_mcp_metrics(conn, &project, retention_days)
        })
    }
    // END_tracker_finish_mcp_request

    // START_CONTRACT_Tracker::record_mcp_call
    // PURPOSE: Persist one completed MCP call metric without active-request lifecycle tracking
    // INPUTS: { tool_name: &str }, { duration_ms: u64 }, { status: McpCallStatus }, { error_message: &str }
    // OUTPUTS: { anyhow::Result<()> }
    // SIDE_EFFECTS: writes to mcp_tool_calls when tracking is enabled
    // LINKS:
    //   -> NFR-003 (traces_to) - MCP tool economics stay separate from token-savings commands
    // START_tracker_record_mcp_call
    pub async fn record_mcp_call(
        &self,
        tool_name: &str,
        duration_ms: u64,
        status: McpCallStatus,
        error_message: &str,
    ) -> anyhow::Result<()> {
        self.finish_mcp_request(None, tool_name, duration_ms, status, error_message)
            .await
    }
    // END_tracker_record_mcp_call

    // START_CONTRACT_Tracker::get_mcp_metrics
    // PURPOSE: Query aggregate MCP runtime metrics for the current project
    // INPUTS: { limit: usize }
    // OUTPUTS: { anyhow::Result<McpMetricsSnapshot> }
    // LINKS:
    //   -> NFR-003 (traces_to) - dashboard observability reads MCP economics without command analytics bloat
    // START_tracker_get_mcp_metrics
    pub async fn get_mcp_metrics(&self, limit: usize) -> anyhow::Result<McpMetricsSnapshot> {
        if !self.config.tracking.enabled() || !self.config.observability.mcp_metrics_enabled() {
            return Ok(McpMetricsSnapshot::default());
        }
        let project = self.project_key();
        self.with_conn(|conn| {
            ensure_mcp_metrics_schema(conn)?;
            query_mcp_metrics(conn, &project, limit)
        })
    }
    // END_tracker_get_mcp_metrics
}

// END_public_api

// START_CONTRACT_ensure_mcp_metrics_schema
// PURPOSE: Ensure MCP metrics tables and indexes exist
// INPUTS: { conn: &rusqlite::Connection }
// OUTPUTS: { rusqlite::Result<()> }
// SIDE_EFFECTS: creates MCP metrics tables and indexes
// LINKS:
//   -> NFR-002 (traces_to) - schema migration is idempotent
// START_ensure_mcp_metrics_schema
fn ensure_mcp_metrics_schema(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS mcp_tool_calls (
            id INTEGER PRIMARY KEY,
            timestamp TEXT NOT NULL DEFAULT (datetime('now')),
            tool_name TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            status TEXT NOT NULL,
            error_message TEXT DEFAULT '',
            project_path TEXT DEFAULT ''
        );
        CREATE TABLE IF NOT EXISTS mcp_active_requests (
            id INTEGER PRIMARY KEY,
            started_at TEXT NOT NULL DEFAULT (datetime('now')),
            tool_name TEXT NOT NULL,
            project_path TEXT DEFAULT ''
        );
        CREATE INDEX IF NOT EXISTS idx_mcp_tool_calls_project_tool
            ON mcp_tool_calls(project_path, tool_name, timestamp);
        CREATE INDEX IF NOT EXISTS idx_mcp_active_requests_project
            ON mcp_active_requests(project_path, started_at);",
    )
}
// END_ensure_mcp_metrics_schema

// START_CONTRACT_insert_mcp_call
// PURPOSE: Insert one completed MCP call metric row
// INPUTS: { conn: &rusqlite::Connection }, { tool_name: &str }, { duration_ms: i64 }, { status: McpCallStatus }, { error_message: &str }, { project: &str }
// OUTPUTS: { rusqlite::Result<()> }
// SIDE_EFFECTS: writes one mcp_tool_calls row
// LINKS:
//   -> NFR-003 (traces_to) - completed MCP call metrics quantify latency and errors
// START_insert_mcp_call
fn insert_mcp_call(
    conn: &rusqlite::Connection,
    tool_name: &str,
    duration_ms: i64,
    status: McpCallStatus,
    error_message: &str,
    project: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO mcp_tool_calls (tool_name, duration_ms, status, error_message, project_path)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![
            tool_name,
            duration_ms,
            status.as_str(),
            error_message,
            project
        ],
    )
    .map(|_| ())
}
// END_insert_mcp_call

// START_CONTRACT_prune_mcp_metrics
// PURPOSE: Delete MCP metric rows older than the configured retention window
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }, { retention_days: u32 }
// OUTPUTS: { rusqlite::Result<()> }
// SIDE_EFFECTS: deletes old mcp_tool_calls and stale mcp_active_requests rows
// LINKS:
//   -> NFR-002 (traces_to) - metrics retention is bounded by configuration
//   -> M-CONFIG (depends) - observability retention setting controls cleanup
// START_prune_mcp_metrics
fn prune_mcp_metrics(
    conn: &rusqlite::Connection,
    project: &str,
    retention_days: u32,
) -> rusqlite::Result<()> {
    let modifier = retention_modifier(retention_days);
    conn.execute(
        "DELETE FROM mcp_tool_calls
         WHERE project_path = ?1 AND timestamp < datetime('now', ?2)",
        rusqlite::params![project, modifier],
    )?;
    conn.execute(
        "DELETE FROM mcp_active_requests
         WHERE project_path = ?1 AND started_at < datetime('now', ?2)",
        rusqlite::params![project, modifier],
    )?;
    Ok(())
}
// END_prune_mcp_metrics

// START_CONTRACT_retention_modifier
// PURPOSE: Convert retention days into a SQLite datetime modifier
// INPUTS: { retention_days: u32 }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - retention values stay positive at query time
// START_retention_modifier
fn retention_modifier(retention_days: u32) -> String {
    format!("-{} days", retention_days.max(1))
}
// END_retention_modifier

// START_CONTRACT_query_mcp_metrics
// PURPOSE: Query aggregate MCP metrics from separate MCP tables
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }, { limit: usize }
// OUTPUTS: { rusqlite::Result<McpMetricsSnapshot> }
// LINKS:
//   -> NFR-003 (traces_to) - MCP stats stay separate from token-savings command analytics
// START_query_mcp_metrics
fn query_mcp_metrics(
    conn: &rusqlite::Connection,
    project: &str,
    limit: usize,
) -> rusqlite::Result<McpMetricsSnapshot> {
    let active_requests = query_active_request_count(conn, project)?;
    let mut snapshot = query_mcp_totals(conn, project)?;
    snapshot.active_requests = active_requests;
    snapshot.tools = query_top_mcp_tools(conn, project, limit)?;
    Ok(snapshot)
}
// END_query_mcp_metrics

// START_CONTRACT_query_active_request_count
// PURPOSE: Count active MCP requests for one project
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }
// OUTPUTS: { rusqlite::Result<u64> }
// LINKS:
//   -> NFR-003 (traces_to) - active MCP request count supports readiness and dashboard observability
// START_query_active_request_count
fn query_active_request_count(conn: &rusqlite::Connection, project: &str) -> rusqlite::Result<u64> {
    conn.query_row(
        "SELECT COUNT(*) FROM mcp_active_requests WHERE project_path = ?1",
        rusqlite::params![project],
        |row| row.get::<_, i64>(0),
    )
    .map(i64_to_u64)
}
// END_query_active_request_count

// START_CONTRACT_query_mcp_totals
// PURPOSE: Query total MCP call counters and latency summary for one project
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }
// OUTPUTS: { rusqlite::Result<McpMetricsSnapshot> }
// LINKS:
//   -> NFR-003 (traces_to) - total MCP call counters support runtime economics dashboards
// START_query_mcp_totals
fn query_mcp_totals(
    conn: &rusqlite::Connection,
    project: &str,
) -> rusqlite::Result<McpMetricsSnapshot> {
    conn.query_row(
        "SELECT
            COUNT(*),
            COALESCE(SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END), 0),
            COALESCE(AVG(duration_ms), 0.0),
            COALESCE(MAX(duration_ms), 0),
            COALESCE(MAX(timestamp), '')
         FROM mcp_tool_calls
         WHERE project_path = ?1",
        rusqlite::params![project],
        |row| {
            Ok(McpMetricsSnapshot {
                total_calls: i64_to_u64(row.get::<_, i64>(0)?),
                active_requests: 0,
                error_calls: i64_to_u64(row.get::<_, i64>(1)?),
                avg_duration_ms: row.get::<_, f64>(2)?,
                max_duration_ms: i64_to_u64(row.get::<_, i64>(3)?),
                last_seen: row.get::<_, String>(4)?,
                tools: Vec::new(),
            })
        },
    )
}
// END_query_mcp_totals

// START_CONTRACT_query_top_mcp_tools
// PURPOSE: Query per-tool MCP metrics ordered by call count
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }, { limit: usize }
// OUTPUTS: { rusqlite::Result<Vec<McpToolMetric>> }
// LINKS:
//   -> NFR-003 (traces_to) - per-tool MCP metrics identify hot and failing tools
// START_query_top_mcp_tools
fn query_top_mcp_tools(
    conn: &rusqlite::Connection,
    project: &str,
    limit: usize,
) -> rusqlite::Result<Vec<McpToolMetric>> {
    let mut stmt = conn.prepare(
        "SELECT
            tool_name,
            COUNT(*) AS calls,
            COALESCE(AVG(duration_ms), 0.0) AS avg_ms,
            COALESCE(MAX(duration_ms), 0) AS max_ms,
            COALESCE(SUM(CASE WHEN status = 'error' THEN 1 ELSE 0 END), 0) AS errors,
            COALESCE(MAX(timestamp), '') AS last_seen
         FROM mcp_tool_calls
         WHERE project_path = ?1
         GROUP BY tool_name
         ORDER BY calls DESC, tool_name ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(
        rusqlite::params![project, u64_to_i64_saturating(limit as u64)],
        |row| {
            Ok(McpToolMetric {
                tool_name: row.get(0)?,
                calls: i64_to_u64(row.get::<_, i64>(1)?),
                avg_duration_ms: row.get::<_, f64>(2)?,
                max_duration_ms: i64_to_u64(row.get::<_, i64>(3)?),
                errors: i64_to_u64(row.get::<_, i64>(4)?),
                last_seen: row.get(5)?,
            })
        },
    )?;
    rows.collect()
}
// END_query_top_mcp_tools

// START_CONTRACT_normalize_tool_name
// PURPOSE: Normalize blank MCP tool names to a stable placeholder
// INPUTS: { tool_name: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-003 (traces_to) - stable tool identity keeps MCP metrics aggregatable
// START_normalize_tool_name
fn normalize_tool_name(tool_name: &str) -> String {
    let trimmed = tool_name.trim();
    if trimmed.is_empty() {
        "unknown".into()
    } else {
        trimmed.chars().take(128).collect()
    }
}
// END_normalize_tool_name

// START_CONTRACT_truncate_error_message
// PURPOSE: Bound stored MCP error messages
// INPUTS: { error_message: &str }
// OUTPUTS: { String }
// LINKS:
//   -> NFR-002 (traces_to) - bounded persistence prevents oversized metrics rows
// START_truncate_error_message
fn truncate_error_message(error_message: &str) -> String {
    error_message.chars().take(512).collect()
}
// END_truncate_error_message

// START_CONTRACT_u64_to_i64_saturating
// PURPOSE: Convert u64 counters to SQLite i64 values without overflow
// INPUTS: { value: u64 }
// OUTPUTS: { i64 }
// LINKS:
//   -> NFR-002 (traces_to) - explicit conversion avoids integer overflow
// START_u64_to_i64_saturating
fn u64_to_i64_saturating(value: u64) -> i64 {
    match i64::try_from(value) {
        Ok(converted) => converted,
        Err(_) => i64::MAX,
    }
}
// END_u64_to_i64_saturating

// START_CONTRACT_i64_to_u64
// PURPOSE: Convert SQLite i64 counters to non-negative u64 values
// INPUTS: { value: i64 }
// OUTPUTS: { u64 }
// LINKS:
//   -> NFR-002 (traces_to) - explicit conversion avoids negative counters in JSON payloads
// START_i64_to_u64
fn i64_to_u64(value: i64) -> u64 {
    match u64::try_from(value) {
        Ok(converted) => converted,
        Err(_) => 0,
    }
}
// END_i64_to_u64

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn tracker_for_test() -> super::super::Tracker {
        let data_home = tempfile::tempdir().expect("data home");
        let project_home = tempfile::tempdir().expect("project home");
        super::super::Tracker::new_for_test(
            &Config::default(),
            data_home.path(),
            project_home.path(),
            Some("mcp-metrics-test"),
        )
    }

    #[tokio::test]
    async fn tracker_records_mcp_metrics_without_command_stats() {
        let tracker = tracker_for_test();

        tracker
            .record_mcp_call("semantic_search", 42, McpCallStatus::Ok, "")
            .await
            .expect("record ok");
        tracker
            .record_mcp_call("semantic_search", 84, McpCallStatus::Error, "boom")
            .await
            .expect("record error");

        let metrics = tracker.get_mcp_metrics(10).await.expect("metrics");
        let token_stats = tracker.get_stats().await.expect("token stats");

        assert_eq!(metrics.total_calls, 2);
        assert_eq!(metrics.error_calls, 1);
        assert_eq!(metrics.tools[0].tool_name, "semantic_search");
        assert_eq!(metrics.tools[0].calls, 2);
        assert_eq!(metrics.tools[0].errors, 1);
        assert_eq!(token_stats.total_commands, 0);
    }

    #[tokio::test]
    async fn tracker_tracks_active_mcp_requests() {
        let tracker = tracker_for_test();

        let active = tracker
            .start_mcp_request("token_savings")
            .await
            .expect("start")
            .expect("active id");
        let active_metrics = tracker.get_mcp_metrics(10).await.expect("active metrics");
        assert_eq!(active_metrics.active_requests, 1);

        tracker
            .finish_mcp_request(Some(active), "token_savings", 12, McpCallStatus::Ok, "")
            .await
            .expect("finish");
        let finished_metrics = tracker.get_mcp_metrics(10).await.expect("finished metrics");

        assert_eq!(finished_metrics.active_requests, 0);
        assert_eq!(finished_metrics.total_calls, 1);
        assert_eq!(finished_metrics.tools[0].tool_name, "token_savings");
    }

    #[tokio::test]
    async fn tracker_skips_mcp_metrics_when_tracking_disabled() {
        let data_home = tempfile::tempdir().expect("data home");
        let project_home = tempfile::tempdir().expect("project home");
        let mut config = Config::default();
        config.tracking.enabled = false;
        let tracker = super::super::Tracker::new_for_test(
            &config,
            data_home.path(),
            project_home.path(),
            Some("mcp-disabled-test"),
        );

        let active = tracker
            .start_mcp_request("semantic_search")
            .await
            .expect("disabled start");
        tracker
            .record_mcp_call("semantic_search", 10, McpCallStatus::Ok, "")
            .await
            .expect("disabled record");
        let metrics = tracker.get_mcp_metrics(10).await.expect("metrics");

        assert!(active.is_none());
        assert_eq!(metrics.total_calls, 0);
        assert_eq!(metrics.active_requests, 0);
    }

    #[tokio::test]
    async fn tracker_skips_mcp_metrics_when_observability_disabled() {
        let data_home = tempfile::tempdir().expect("data home");
        let project_home = tempfile::tempdir().expect("project home");
        let mut config = Config::default();
        config.observability.mcp_metrics_enabled = false;
        let tracker = super::super::Tracker::new_for_test(
            &config,
            data_home.path(),
            project_home.path(),
            Some("mcp-observability-disabled-test"),
        );

        let active = tracker
            .start_mcp_request("semantic_search")
            .await
            .expect("disabled start");
        tracker
            .record_mcp_call("semantic_search", 10, McpCallStatus::Ok, "")
            .await
            .expect("disabled record");
        let metrics = tracker.get_mcp_metrics(10).await.expect("metrics");

        assert!(active.is_none());
        assert_eq!(metrics.total_calls, 0);
        assert_eq!(metrics.active_requests, 0);
    }

    #[test]
    fn retention_modifier_keeps_zero_days_positive() {
        assert_eq!(retention_modifier(0), "-1 days");
        assert_eq!(retention_modifier(30), "-30 days");
    }
}
