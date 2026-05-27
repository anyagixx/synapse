// MODULE_CONTRACT
// MODULE_ID: M-TRACKING
// PURPOSE: Estimate current session context-window pressure from tracked token usage.
// SCOPE: PressureLevel, ContextPressure, Tracker::context_pressure, and scoped session input/output aggregation.
// DEPENDS: M-TRACKING, M-CONFIG
// LINKS:
//   -> M-TRACKING (depends) - reads commands table through Tracker connection scope
//   -> M-CONFIG (depends) - consumes configured context_window_limit supplied by callers
//   -> NFR-003 (traces_to) - context pressure protects token economy during long sessions

// START_MODULE_MAP
// PressureLevel - Low, moderate, high, or critical context pressure level
// ContextPressure - Per-session context window pressure report
// Tracker::context_pressure - Estimate current session context pressure
// session_context_totals - Query input/output token totals for a scoped session
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added context pressure estimates]
// END_CHANGE_SUMMARY

use super::Tracker;
use serde::{Deserialize, Serialize};

const MODERATE_PRESSURE_PCT: f64 = 60.0;
const HIGH_PRESSURE_PCT: f64 = 80.0;
const CRITICAL_PRESSURE_PCT: f64 = 95.0;

// START_public_api

// START_PressureLevel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PressureLevel {
    Low,
    Moderate,
    High,
    Critical,
}
// END_PressureLevel

// START_ContextPressure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextPressure {
    pub session_id: String,
    pub estimated_context_tokens: u64,
    pub context_window_limit: u64,
    pub pressure_pct: f64,
    pub level: PressureLevel,
    pub recommendation: String,
    pub headroom: u64,
}
// END_ContextPressure

impl Tracker {
    // START_CONTRACT_Tracker::context_pressure
    // PURPOSE: Estimate current session context pressure from input/output token totals and configured context limit.
    // INPUTS: { context_limit: u64 }
    // OUTPUTS: { anyhow::Result<ContextPressure> }
    // LINKS:
    //   -> NFR-003 (traces_to) - pressure status estimates when to compact, go terse, or resume in a new session
    // START_tracker_context_pressure
    pub async fn context_pressure(&self, context_limit: u64) -> anyhow::Result<ContextPressure> {
        if context_limit == 0 {
            anyhow::bail!("context_limit must be greater than 0");
        }

        let session_id = self.session_id();
        let project = self.project_key();
        let (input_tokens, output_tokens) =
            self.with_conn(|conn| session_context_totals(conn, &project, &session_id))?;
        let overhead = context_limit / 7;
        let estimated_context_tokens = overhead
            .saturating_add(input_tokens)
            .saturating_add(output_tokens);
        let pressure_pct =
            ((estimated_context_tokens as f64 / context_limit as f64) * 1000.0).round() / 10.0;
        let headroom = context_limit.saturating_sub(estimated_context_tokens);
        let (level, recommendation) = pressure_recommendation(pressure_pct);

        Ok(ContextPressure {
            session_id,
            estimated_context_tokens,
            context_window_limit: context_limit,
            pressure_pct,
            level,
            recommendation,
            headroom,
        })
    }
    // END_tracker_context_pressure
}

// START_CONTRACT_pressure_recommendation
// PURPOSE: Map context pressure percentage to stable level and action text.
// INPUTS: { pressure_pct: f64 }
// OUTPUTS: { (PressureLevel, String) }
// START_pressure_recommendation
fn pressure_recommendation(pressure_pct: f64) -> (PressureLevel, String) {
    if pressure_pct >= CRITICAL_PRESSURE_PCT {
        (
            PressureLevel::Critical,
            "Context critically full. Save state with agent_resume and start a new session immediately.".into(),
        )
    } else if pressure_pct >= HIGH_PRESSURE_PCT {
        (
            PressureLevel::High,
            "Context near limit. Compact evidence, use terse responses, and prepare to resume in a new session.".into(),
        )
    } else if pressure_pct >= MODERATE_PRESSURE_PCT {
        (
            PressureLevel::Moderate,
            "Context filling. Prefer terse responses, max_tokens limits, and smaller searches."
                .into(),
        )
    } else {
        (
            PressureLevel::Low,
            "Context healthy. There is room for normal exploration.".into(),
        )
    }
}
// END_pressure_recommendation

// START_CONTRACT_session_context_totals
// PURPOSE: Query input and output token totals for one scoped project/session pair.
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }, { session_id: &str }
// OUTPUTS: { rusqlite::Result<(u64, u64)> }
// START_session_context_totals
fn session_context_totals(
    conn: &rusqlite::Connection,
    project: &str,
    session_id: &str,
) -> rusqlite::Result<(u64, u64)> {
    conn.query_row(
        "SELECT COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0)
         FROM commands
         WHERE project_path = ?1 AND session_id = ?2",
        rusqlite::params![project, session_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )
}
// END_session_context_totals

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    // START_CONTRACT_pressure_tracker
    // PURPOSE: Create an isolated tracker for context pressure tests.
    // OUTPUTS: { (tempfile::TempDir, tempfile::TempDir, Tracker) }
    // START_pressure_tracker
    fn pressure_tracker(session_id: &str) -> (tempfile::TempDir, tempfile::TempDir, Tracker) {
        let data_home = tempfile::tempdir().expect("data home");
        let project_root = tempfile::tempdir().expect("project root");
        let tracker = Tracker::new_for_test(
            &Config::default(),
            data_home.path(),
            project_root.path(),
            Some(session_id),
        );
        (data_home, project_root, tracker)
    }
    // END_pressure_tracker

    // START_CONTRACT_context_pressure_reports_low_moderate_high_and_critical
    // PURPOSE: Verify context_pressure reports all four pressure thresholds from current session totals.
    // START_context_pressure_reports_low_moderate_high_and_critical
    #[tokio::test]
    async fn context_pressure_reports_low_moderate_high_and_critical() {
        let (_data_home, _project_root, low) = pressure_tracker("pressure-low");
        low.record("low", 100, 0).await.unwrap();
        let low_status = low.context_pressure(1_000).await.unwrap();
        assert_eq!(low_status.level, PressureLevel::Low);
        assert_eq!(low_status.estimated_context_tokens, 242);
        assert_eq!(low_status.headroom, 758);

        let (_data_home, _project_root, moderate) = pressure_tracker("pressure-moderate");
        moderate.record("moderate", 500, 0).await.unwrap();
        let moderate_status = moderate.context_pressure(1_000).await.unwrap();
        assert_eq!(moderate_status.level, PressureLevel::Moderate);

        let (_data_home, _project_root, high) = pressure_tracker("pressure-high");
        high.record("high", 700, 0).await.unwrap();
        let high_status = high.context_pressure(1_000).await.unwrap();
        assert_eq!(high_status.level, PressureLevel::High);

        let (_data_home, _project_root, critical) = pressure_tracker("pressure-critical");
        critical.record("critical", 820, 0).await.unwrap();
        let critical_status = critical.context_pressure(1_000).await.unwrap();
        assert_eq!(critical_status.level, PressureLevel::Critical);
    }
    // END_context_pressure_reports_low_moderate_high_and_critical

    // START_CONTRACT_context_pressure_rejects_zero_limit
    // PURPOSE: Verify context_pressure refuses a zero context limit to avoid division by zero.
    // START_context_pressure_rejects_zero_limit
    #[tokio::test]
    async fn context_pressure_rejects_zero_limit() {
        let (_data_home, _project_root, tracker) = pressure_tracker("pressure-zero");

        assert!(tracker.context_pressure(0).await.is_err());
    }
    // END_context_pressure_rejects_zero_limit
}
