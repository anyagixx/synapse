// MODULE_CONTRACT
// MODULE_ID: M-RUNNER
// PURPOSE: Compact run evidence references for token-efficient persisted run state without changing plain save semantics.
// SCOPE: EvidenceCompactionReport, evidence reference deduplication, prefix aliasing, long-list truncation, save_with_compaction, and persisted run compaction entry point.
// DEPENDS: M-RUNNER
// LINKS:
//   -> M-RUNNER (depends) - extends RunManager and RunRecord behavior
//   -> NFR-003 (traces_to) - compact evidence reduces repeated context overhead
//   <- V-M-RUNNER (verified_by) - runtime verification shard

// START_MODULE_MAP
// EvidenceCompactionReport - Summary of before/after evidence compaction savings
// RunManager::compact_evidence - Deduplicate, alias, and truncate evidence refs in memory
// RunManager::save_with_compaction - Compact a run record and persist it atomically
// RunManager::compact_run_evidence - Load, compact, and persist one run by id
// evidence_aliases - Common evidence prefix aliases
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added evidence compaction entry points]
// END_CHANGE_SUMMARY

use super::{RunManager, RunRecord};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

const MAX_VISIBLE_EVIDENCE_REFS: usize = 20;

// START_public_api

// START_EvidenceCompactionReport
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EvidenceCompactionReport {
    pub before_count: usize,
    pub after_count: usize,
    pub before_chars: usize,
    pub after_chars: usize,
    pub duplicates_removed: usize,
    pub aliases_applied: usize,
    pub truncated_refs: usize,
    pub savings_pct: f64,
}
// END_EvidenceCompactionReport

impl EvidenceCompactionReport {
    // START_CONTRACT_EvidenceCompactionReport::changed
    // PURPOSE: Return true when compaction changed count, aliases, truncation, or character footprint.
    // OUTPUTS: { bool }
    // START_evidence_compaction_report_changed
    pub fn changed(&self) -> bool {
        self.duplicates_removed > 0
            || self.aliases_applied > 0
            || self.truncated_refs > 0
            || self.after_chars < self.before_chars
    }
    // END_evidence_compaction_report_changed
}

impl RunManager {
    // START_CONTRACT_RunManager::compact_evidence
    // PURPOSE: Deduplicate, alias, and truncate one run record's evidence refs in memory.
    // INPUTS: { record: &mut RunRecord }
    // OUTPUTS: { EvidenceCompactionReport }
    // LINKS:
    //   -> NFR-003 (traces_to) - compact refs preserve evidence while reducing token overhead
    // START_run_manager_compact_evidence
    pub fn compact_evidence(record: &mut RunRecord) -> EvidenceCompactionReport {
        let before_count = record.evidence_refs.len();
        let before_chars = evidence_chars(&record.evidence_refs);
        let mut seen = HashSet::new();

        record
            .evidence_refs
            .retain(|reference| seen.insert(reference.clone()));
        let deduped_count = record.evidence_refs.len();
        let duplicates_removed = before_count.saturating_sub(deduped_count);
        let aliases_applied = apply_aliases(&mut record.evidence_refs);
        let truncated_refs = truncate_evidence_refs(&mut record.evidence_refs);
        let after_count = record.evidence_refs.len();
        let after_chars = evidence_chars(&record.evidence_refs);

        EvidenceCompactionReport {
            before_count,
            after_count,
            before_chars,
            after_chars,
            duplicates_removed,
            aliases_applied,
            truncated_refs,
            savings_pct: savings_pct(before_chars, after_chars),
        }
    }
    // END_run_manager_compact_evidence

    // START_CONTRACT_RunManager::save_with_compaction
    // PURPOSE: Compact one run record's evidence refs and persist it through the existing atomic save path.
    // INPUTS: { record: &mut RunRecord }
    // OUTPUTS: { anyhow::Result<EvidenceCompactionReport> }
    // SIDE_EFFECTS: writes docs/runs/<run_id>.json
    // LINKS:
    //   -> M-RUNNER (depends) - delegates persistence to RunManager::save
    // START_run_manager_save_with_compaction
    pub fn save_with_compaction(
        &self,
        record: &mut RunRecord,
    ) -> anyhow::Result<EvidenceCompactionReport> {
        let report = Self::compact_evidence(record);
        if report.changed() {
            tracing::debug!(
                before_count = report.before_count,
                after_count = report.after_count,
                before_chars = report.before_chars,
                after_chars = report.after_chars,
                savings_pct = report.savings_pct,
                "Evidence compacted"
            );
        }
        self.save(record)?;
        Ok(report)
    }
    // END_run_manager_save_with_compaction

    // START_CONTRACT_RunManager::compact_run_evidence
    // PURPOSE: Load, compact, and persist one existing run record by run id.
    // INPUTS: { run_id: &str }
    // OUTPUTS: { anyhow::Result<EvidenceCompactionReport> }
    // SIDE_EFFECTS: reads and writes docs/runs/<run_id>.json
    // LINKS:
    //   -> M-RUNNER (depends) - reuses load and save_with_compaction
    // START_run_manager_compact_run_evidence
    pub fn compact_run_evidence(&self, run_id: &str) -> anyhow::Result<EvidenceCompactionReport> {
        let mut record = self.load(run_id)?;
        self.save_with_compaction(&mut record)
    }
    // END_run_manager_compact_run_evidence
}

// START_CONTRACT_evidence_chars
// PURPOSE: Count total evidence reference characters for savings reports.
// INPUTS: { refs: &[String] }
// OUTPUTS: { usize }
// START_evidence_chars
fn evidence_chars(refs: &[String]) -> usize {
    refs.iter().map(String::len).sum()
}
// END_evidence_chars

// START_CONTRACT_apply_aliases
// PURPOSE: Replace known long evidence prefixes with compact aliases.
// INPUTS: { refs: &mut [String] }
// OUTPUTS: { usize }
// START_apply_aliases
fn apply_aliases(refs: &mut [String]) -> usize {
    let aliases = evidence_aliases();
    let mut applied = 0;

    for reference in refs {
        if let Some((prefix, alias)) = aliases
            .iter()
            .find(|(prefix, _)| reference.starts_with(prefix))
        {
            *reference = format!("{}{}", alias, &reference[prefix.len()..]);
            applied += 1;
        }
    }

    applied
}
// END_apply_aliases

// START_CONTRACT_truncate_evidence_refs
// PURPOSE: Keep the latest evidence refs when a run stores more than the visible evidence limit.
// INPUTS: { refs: &mut Vec<String> }
// OUTPUTS: { usize }
// START_truncate_evidence_refs
fn truncate_evidence_refs(refs: &mut Vec<String>) -> usize {
    if refs.len() <= MAX_VISIBLE_EVIDENCE_REFS {
        return 0;
    }

    let removed = refs.len() - MAX_VISIBLE_EVIDENCE_REFS;
    let kept = refs
        .iter()
        .skip(removed)
        .take(MAX_VISIBLE_EVIDENCE_REFS)
        .cloned()
        .collect::<Vec<_>>();
    refs.clear();
    refs.push(format!("... {removed} earlier refs compacted"));
    refs.extend(kept);
    removed
}
// END_truncate_evidence_refs

// START_CONTRACT_savings_pct
// PURPOSE: Compute one-decimal percentage savings from before and after character counts.
// INPUTS: { before_chars: usize }, { after_chars: usize }
// OUTPUTS: { f64 }
// START_savings_pct
fn savings_pct(before_chars: usize, after_chars: usize) -> f64 {
    if before_chars == 0 {
        return 0.0;
    }
    let saved = before_chars.saturating_sub(after_chars) as f64;
    ((saved / before_chars as f64) * 1000.0).round() / 10.0
}
// END_savings_pct

// START_CONTRACT_evidence_aliases
// PURPOSE: Return the ordered evidence prefix alias table.
// OUTPUTS: { &'static [(&'static str, &'static str)] }
// START_evidence_aliases
fn evidence_aliases() -> &'static [(&'static str, &'static str)] {
    &[
        ("action://", "▶"),
        ("scenario://", "🎬"),
        ("file://", "📄"),
        ("docs/runs/", "📁"),
        ("docs/tests/results/", "🧪"),
        ("docs/verification/", "🔍"),
        ("docs/modules/", "📦"),
        ("docs/phases/", "📋"),
    ]
}
// END_evidence_aliases

// END_public_api

#[cfg(test)]
mod tests {
    use super::*;

    // START_CONTRACT_sample_run
    // PURPOSE: Build a run record for evidence compaction tests.
    // OUTPUTS: { RunRecord }
    // START_sample_run
    fn sample_run() -> RunRecord {
        RunRecord::new(
            "goal".into(),
            "Phase-87".into(),
            "M-RUNNER".into(),
            "compact evidence".into(),
        )
    }
    // END_sample_run

    // START_CONTRACT_compact_evidence_deduplicates_and_aliases_known_prefixes
    // PURPOSE: Verify duplicate refs are removed and known evidence prefixes are aliased.
    // START_compact_evidence_deduplicates_and_aliases_known_prefixes
    #[test]
    fn compact_evidence_deduplicates_and_aliases_known_prefixes() {
        let mut record = sample_run();
        record.evidence_refs = vec![
            "action://read-artifacts".into(),
            "action://read-artifacts".into(),
            "scenario://blocked-review".into(),
            "docs/runs/run-1/evidence.log".into(),
            "docs/verification/V-M-RUNNER.xml".into(),
            "docs/modules/M-RUNNER.xml".into(),
            "docs/phases/Phase-87.xml".into(),
        ];

        let report = RunManager::compact_evidence(&mut record);

        assert_eq!(report.before_count, 7);
        assert_eq!(report.after_count, 6);
        assert_eq!(report.duplicates_removed, 1);
        assert_eq!(report.aliases_applied, 6);
        assert!(report.savings_pct > 0.0);
        assert_eq!(record.evidence_refs[0], "▶read-artifacts");
        assert!(record.evidence_refs.contains(&"🎬blocked-review".into()));
        assert!(record
            .evidence_refs
            .contains(&"📁run-1/evidence.log".into()));
        assert!(record.evidence_refs.contains(&"🔍V-M-RUNNER.xml".into()));
        assert!(record.evidence_refs.contains(&"📦M-RUNNER.xml".into()));
        assert!(record.evidence_refs.contains(&"📋Phase-87.xml".into()));
    }
    // END_compact_evidence_deduplicates_and_aliases_known_prefixes

    // START_CONTRACT_compact_evidence_truncates_above_visible_limit
    // PURPOSE: Verify large evidence lists keep the newest refs and add a summary marker.
    // START_compact_evidence_truncates_above_visible_limit
    #[test]
    fn compact_evidence_truncates_above_visible_limit() {
        let mut record = sample_run();
        record.evidence_refs = (0..25)
            .map(|index| format!("action://step-{index}"))
            .collect();

        let report = RunManager::compact_evidence(&mut record);

        assert_eq!(report.truncated_refs, 5);
        assert_eq!(record.evidence_refs.len(), 21);
        assert_eq!(record.evidence_refs[0], "... 5 earlier refs compacted");
        assert!(!record.evidence_refs.contains(&"▶step-0".into()));
        assert!(record.evidence_refs.contains(&"▶step-24".into()));
    }
    // END_compact_evidence_truncates_above_visible_limit

    // START_CONTRACT_save_with_compaction_persists_report_and_compacted_record
    // PURPOSE: Verify save_with_compaction writes the compacted record and returns savings report data.
    // START_save_with_compaction_persists_report_and_compacted_record
    #[test]
    fn save_with_compaction_persists_report_and_compacted_record() {
        let root = tempfile::tempdir().expect("temp root");
        let manager = RunManager::new(root.path());
        let mut record = sample_run();
        record.evidence_refs = vec![
            "action://read".into(),
            "action://read".into(),
            "docs/runs/run-1/evidence.log".into(),
        ];

        let report = manager
            .save_with_compaction(&mut record)
            .expect("save with compaction");
        let restored = manager.load(&record.run_id).expect("load compacted run");

        assert!(report.changed());
        assert_eq!(restored.evidence_refs, record.evidence_refs);
        assert_eq!(
            restored.evidence_refs,
            vec!["▶read".to_string(), "📁run-1/evidence.log".to_string()]
        );
    }
    // END_save_with_compaction_persists_report_and_compacted_record

    // START_CONTRACT_plain_save_preserves_uncompacted_evidence_refs
    // PURPOSE: Verify plain save remains backward-compatible and does not compact evidence.
    // START_plain_save_preserves_uncompacted_evidence_refs
    #[test]
    fn plain_save_preserves_uncompacted_evidence_refs() {
        let root = tempfile::tempdir().expect("temp root");
        let manager = RunManager::new(root.path());
        let mut record = sample_run();
        record.evidence_refs = vec!["action://read".into(), "action://read".into()];

        manager.save(&record).expect("plain save");
        let restored = manager.load(&record.run_id).expect("load plain run");

        assert_eq!(restored.evidence_refs, record.evidence_refs);
    }
    // END_plain_save_preserves_uncompacted_evidence_refs
}
