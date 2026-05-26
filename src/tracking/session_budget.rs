// MODULE_CONTRACT
// MODULE_ID: M-TRACKING
// PURPOSE: Query per-session token budget state from recorded command token usage.
// SCOPE: BudgetLevel, BudgetStatus, Tracker::check_budget, Tracker::can_afford, and scoped session token aggregation.
// DEPENDS: M-TRACKING, M-CONFIG
// LINKS:
//   -> M-TRACKING (depends) - reads commands table through Tracker connection scope
//   -> M-CONFIG (depends) - consumes configured budget thresholds supplied by callers
//   -> NFR-003 (traces_to) - budget visibility protects token economy during long sessions

// START_MODULE_MAP
// BudgetLevel - Normal, warning, or blocked budget level
// BudgetStatus - Per-session budget usage report
// Tracker::check_budget - Calculate current session budget status
// Tracker::can_afford - Check whether an estimated operation fits remaining budget
// session_token_totals - Query input/output token totals for a scoped session
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added per-session token budget queries]
// END_CHANGE_SUMMARY

use super::Tracker;
use serde::{Deserialize, Serialize};

// START_public_api

// START_BudgetLevel
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BudgetLevel {
    Normal,
    Warning,
    Blocked,
}
// END_BudgetLevel

// START_BudgetStatus
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BudgetStatus {
    pub session_id: String,
    pub limit: u64,
    pub used_input_tokens: u64,
    pub used_output_tokens: u64,
    pub remaining: u64,
    pub pct_used: f64,
    pub status: BudgetLevel,
    pub message: String,
}
// END_BudgetStatus

impl Tracker {
    // START_CONTRACT_Tracker::check_budget
    // PURPOSE: Return current session budget usage and normal/warning/blocked threshold state.
    // INPUTS: { limit: u64 }, { warn_pct: u64 }, { block_pct: u64 }
    // OUTPUTS: { anyhow::Result<BudgetStatus> }
    // LINKS:
    //   -> NFR-003 (traces_to) - budget status uses input tokens as the spend metric
    // START_tracker_check_budget
    pub async fn check_budget(
        &self,
        limit: u64,
        warn_pct: u64,
        block_pct: u64,
    ) -> anyhow::Result<BudgetStatus> {
        let session_id = self.session_id();
        if limit == 0 {
            return Ok(BudgetStatus {
                session_id,
                limit,
                used_input_tokens: 0,
                used_output_tokens: 0,
                remaining: 0,
                pct_used: 0.0,
                status: BudgetLevel::Normal,
                message: "Token budget is unlimited (limit = 0).".into(),
            });
        }

        let project = self.project_key();
        let (used_input_tokens, used_output_tokens) =
            self.with_conn(|conn| session_token_totals(conn, &project, &session_id))?;
        let remaining = limit.saturating_sub(used_input_tokens);
        let pct_used = ((used_input_tokens as f64 / limit as f64) * 1000.0).round() / 10.0;
        let block_pct = block_pct.clamp(1, 100);
        let warn_pct = warn_pct.min(block_pct);
        let (status, message) = if pct_used >= block_pct as f64 {
            (
                BudgetLevel::Blocked,
                format!(
                    "Token budget exhausted: {}/{} input tokens used ({:.1}%).",
                    used_input_tokens, limit, pct_used
                ),
            )
        } else if pct_used >= warn_pct as f64 {
            (
                BudgetLevel::Warning,
                format!(
                    "Token budget warning: {}/{} input tokens used ({:.1}%). {} remaining.",
                    used_input_tokens, limit, pct_used, remaining
                ),
            )
        } else {
            (
                BudgetLevel::Normal,
                format!(
                    "Token budget: {}/{} input tokens used ({:.1}%). {} remaining.",
                    used_input_tokens, limit, pct_used, remaining
                ),
            )
        };

        Ok(BudgetStatus {
            session_id,
            limit,
            used_input_tokens,
            used_output_tokens,
            remaining,
            pct_used,
            status,
            message,
        })
    }
    // END_tracker_check_budget

    // START_CONTRACT_Tracker::can_afford
    // PURPOSE: Return whether an estimated operation can fit within the current session budget.
    // INPUTS: { estimated_tokens: u64 }, { limit: u64 }, { block_pct: u64 }
    // OUTPUTS: { anyhow::Result<bool> }
    // LINKS:
    //   -> NFR-003 (traces_to) - expensive operations can gate themselves before spending context
    // START_tracker_can_afford
    pub async fn can_afford(
        &self,
        estimated_tokens: u64,
        limit: u64,
        block_pct: u64,
    ) -> anyhow::Result<bool> {
        if limit == 0 {
            return Ok(true);
        }
        let status = self.check_budget(limit, 80, block_pct).await?;
        if status.status == BudgetLevel::Blocked {
            return Ok(false);
        }
        Ok(status.remaining >= estimated_tokens)
    }
    // END_tracker_can_afford
}

// START_CONTRACT_session_token_totals
// PURPOSE: Query input and output token totals for one scoped project/session pair.
// INPUTS: { conn: &rusqlite::Connection }, { project: &str }, { session_id: &str }
// OUTPUTS: { rusqlite::Result<(u64, u64)> }
// START_session_token_totals
fn session_token_totals(
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
// END_session_token_totals

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    // START_CONTRACT_budget_tracker
    // PURPOSE: Create an isolated tracker for budget tests.
    // OUTPUTS: { (tempfile::TempDir, tempfile::TempDir, Tracker) }
    // START_budget_tracker
    fn budget_tracker() -> (tempfile::TempDir, tempfile::TempDir, Tracker) {
        let data_home = tempfile::tempdir().expect("data home");
        let project_root = tempfile::tempdir().expect("project root");
        let tracker = Tracker::new_for_test(
            &Config::default(),
            data_home.path(),
            project_root.path(),
            Some("budget-session"),
        );
        (data_home, project_root, tracker)
    }
    // END_budget_tracker

    // START_CONTRACT_check_budget_reports_normal_warning_and_blocked
    // PURPOSE: Verify check_budget reports normal, warning, and blocked states from current session input tokens.
    // START_check_budget_reports_normal_warning_and_blocked
    #[tokio::test]
    async fn check_budget_reports_normal_warning_and_blocked() {
        let (_data_home, _project_root, tracker) = budget_tracker();
        tracker.record("first", 50, 10).await.unwrap();

        let normal = tracker.check_budget(100, 80, 100).await.unwrap();
        assert_eq!(normal.status, BudgetLevel::Normal);
        assert_eq!(normal.used_input_tokens, 50);
        assert_eq!(normal.used_output_tokens, 10);
        assert_eq!(normal.remaining, 50);

        tracker.record("second", 35, 5).await.unwrap();
        let warning = tracker.check_budget(100, 80, 100).await.unwrap();
        assert_eq!(warning.status, BudgetLevel::Warning);
        assert_eq!(warning.used_input_tokens, 85);

        tracker.record("third", 20, 5).await.unwrap();
        let blocked = tracker.check_budget(100, 80, 100).await.unwrap();
        assert_eq!(blocked.status, BudgetLevel::Blocked);
        assert_eq!(blocked.used_input_tokens, 105);
        assert_eq!(blocked.remaining, 0);
    }
    // END_check_budget_reports_normal_warning_and_blocked

    // START_CONTRACT_budget_limit_zero_is_unlimited
    // PURPOSE: Verify limit zero stays unlimited and backward-compatible.
    // START_budget_limit_zero_is_unlimited
    #[tokio::test]
    async fn budget_limit_zero_is_unlimited() {
        let (_data_home, _project_root, tracker) = budget_tracker();
        tracker.record("expensive", 1_000_000, 10).await.unwrap();

        let status = tracker.check_budget(0, 80, 100).await.unwrap();
        let can_afford = tracker.can_afford(u64::MAX, 0, 100).await.unwrap();

        assert_eq!(status.status, BudgetLevel::Normal);
        assert_eq!(status.limit, 0);
        assert_eq!(status.used_input_tokens, 0);
        assert!(can_afford);
    }
    // END_budget_limit_zero_is_unlimited

    // START_CONTRACT_can_afford_respects_remaining_tokens
    // PURPOSE: Verify can_afford compares estimated tokens to remaining session budget.
    // START_can_afford_respects_remaining_tokens
    #[tokio::test]
    async fn can_afford_respects_remaining_tokens() {
        let (_data_home, _project_root, tracker) = budget_tracker();
        tracker.record("used", 80, 20).await.unwrap();

        assert!(tracker.can_afford(20, 100, 100).await.unwrap());
        assert!(!tracker.can_afford(21, 100, 100).await.unwrap());
    }
    // END_can_afford_respects_remaining_tokens
}
