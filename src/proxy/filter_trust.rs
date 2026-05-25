// MODULE_CONTRACT
// MODULE_ID: M-PROXY-FILTER
// PURPOSE: Project filter trust store — gates .synapse/filters.toml execution by reviewed file hash
// SCOPE: TrustStatus, TrustEntry, project filter path resolution, SHA-256 trust/untrust/status operations
// DEPENDS: N/A
// LINKS:
//   → Phase-24 (implements) - RTK-style trust-before-load for project-local filters
//   ← V-M-PROXY-FILTER (verified_by) - trust gate verification

// START_MODULE_MAP
// TrustStatus — Trust decision for a project-local filter file
// TrustEntry — Persisted trusted path/hash record
// check_trust — Compares current project filter hash against trust store
// trust_project_filters — Stores current project filter hash as trusted
// untrust_project_filters — Removes current project filter trust record
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.1.0 — Replaced trust-store test env mutation with scoped guards]
// END_CHANGE_SUMMARY

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

// START_public_api

// START_TrustStatus
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustStatus {
    Missing,
    Trusted,
    Untrusted,
    ContentChanged {
        trusted_sha256: String,
        current_sha256: String,
    },
    EnvOverride,
}

impl TrustStatus {
    // START_CONTRACT_TrustStatus::label
    // PURPOSE: Return a stable human-readable trust status label
    // OUTPUTS: { &'static str }
    // START_trust_status_label
    pub fn label(&self) -> &'static str {
        match self {
            TrustStatus::Missing => "missing",
            TrustStatus::Trusted => "trusted",
            TrustStatus::Untrusted => "untrusted",
            TrustStatus::ContentChanged { .. } => "content-changed",
            TrustStatus::EnvOverride => "env-override",
        }
    }
    // END_trust_status_label
}
// END_TrustStatus

// START_TrustEntry
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct TrustEntry {
    pub path: String,
    pub sha256: String,
    pub trusted_at_unix: u64,
}
// END_TrustEntry

// START_CONTRACT_project_filter_path
// PURPOSE: Return the canonical project-local Synapse filter file path relative to current working directory
// OUTPUTS: { PathBuf }
// START_project_filter_path
pub fn project_filter_path() -> PathBuf {
    Path::new(".synapse").join("filters.toml")
}
// END_project_filter_path

// START_CONTRACT_check_trust
// PURPOSE: Check whether a project-local filter file is trusted for execution
// INPUTS: { path: &Path }
// OUTPUTS: { anyhow::Result<TrustStatus> }
// LINKS:
//   → Phase-24 (implements) - project filters are skipped unless trusted
// START_check_trust
pub fn check_trust(path: &Path) -> anyhow::Result<TrustStatus> {
    let store_path = store_path()?;
    check_trust_with_store(path, &store_path)
}

pub(crate) fn check_trust_with_store(
    path: &Path,
    store_path: &Path,
) -> anyhow::Result<TrustStatus> {
    if !path.exists() {
        return Ok(TrustStatus::Missing);
    }
    if env_trust_override() {
        return Ok(TrustStatus::EnvOverride);
    }

    let key = trust_key(path)?;
    let current_sha256 = file_sha256(path)?;
    let store = read_store_at(store_path)?;
    match store.trusted.get(&key) {
        Some(entry) if entry.sha256 == current_sha256 => Ok(TrustStatus::Trusted),
        Some(entry) => Ok(TrustStatus::ContentChanged {
            trusted_sha256: entry.sha256.clone(),
            current_sha256,
        }),
        None => Ok(TrustStatus::Untrusted),
    }
}
// END_check_trust

// START_CONTRACT_trust_project_filters
// PURPOSE: Trust the current project's .synapse/filters.toml by storing its SHA-256 hash
// OUTPUTS: { anyhow::Result<TrustEntry> }
// SIDE_EFFECTS: writes Synapse filter trust store under XDG data directory
// START_trust_project_filters
pub fn trust_project_filters() -> anyhow::Result<TrustEntry> {
    trust_path(&project_filter_path())
}
// END_trust_project_filters

// START_CONTRACT_untrust_project_filters
// PURPOSE: Remove trust for the current project's .synapse/filters.toml
// OUTPUTS: { anyhow::Result<bool> - true when an entry was removed }
// SIDE_EFFECTS: rewrites Synapse filter trust store
// START_untrust_project_filters
pub fn untrust_project_filters() -> anyhow::Result<bool> {
    untrust_path(&project_filter_path())
}
// END_untrust_project_filters

// START_CONTRACT_project_filter_status
// PURPOSE: Return trust status for the current project's filter file
// OUTPUTS: { anyhow::Result<TrustStatus> }
// START_project_filter_status
pub fn project_filter_status() -> anyhow::Result<TrustStatus> {
    check_trust(&project_filter_path())
}
// END_project_filter_status

// END_public_api

#[derive(Debug, Default, serde::Deserialize, serde::Serialize)]
struct TrustStore {
    trusted: BTreeMap<String, TrustEntry>,
}

fn trust_path(path: &Path) -> anyhow::Result<TrustEntry> {
    let store_path = store_path()?;
    trust_path_with_store(path, &store_path)
}

pub(crate) fn trust_path_with_store(path: &Path, store_path: &Path) -> anyhow::Result<TrustEntry> {
    if !path.exists() {
        anyhow::bail!("project filter file not found: {}", path.display());
    }
    let key = trust_key(path)?;
    let entry = TrustEntry {
        path: key.clone(),
        sha256: file_sha256(path)?,
        trusted_at_unix: unix_now(),
    };
    let mut store = read_store_at(store_path)?;
    store.trusted.insert(key, entry.clone());
    write_store_at(store_path, &store)?;
    Ok(entry)
}

fn untrust_path(path: &Path) -> anyhow::Result<bool> {
    let store_path = store_path()?;
    untrust_path_with_store(path, &store_path)
}

fn untrust_path_with_store(path: &Path, store_path: &Path) -> anyhow::Result<bool> {
    if !path.exists() {
        return Ok(false);
    }
    let key = trust_key(path)?;
    let mut store = read_store_at(store_path)?;
    let removed = store.trusted.remove(&key).is_some();
    write_store_at(store_path, &store)?;
    Ok(removed)
}

fn trust_key(path: &Path) -> anyhow::Result<String> {
    let canonical = std::fs::canonicalize(path)?;
    Ok(canonical.to_string_lossy().to_string())
}

fn file_sha256(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    Ok(digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>())
}

fn read_store_at(path: &Path) -> anyhow::Result<TrustStore> {
    if !path.exists() {
        return Ok(TrustStore::default());
    }
    let content = std::fs::read_to_string(path)?;
    let store = serde_json::from_str(&content)?;
    Ok(store)
}

fn write_store_at(path: &Path, store: &TrustStore) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(store)?)?;
    Ok(())
}

fn store_path() -> anyhow::Result<PathBuf> {
    let data_dir = dirs::data_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine user data directory"))?;
    Ok(data_dir.join("synapse").join("filter-trust.json"))
}

fn env_trust_override() -> bool {
    std::env::var("SYNAPSE_TRUST_PROJECT_FILTERS")
        .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "YES"))
        .unwrap_or(false)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_status_changes_after_file_update() {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("filter-trust.json");
        let filter_path = dir.path().join(".synapse").join("filters.toml");

        std::fs::create_dir(dir.path().join(".synapse")).unwrap();
        std::fs::write(&filter_path, "[[filters]]\nmatch_command = \"x\"\n").unwrap();

        assert_eq!(
            check_trust_with_store(&filter_path, &store_path).unwrap(),
            TrustStatus::Untrusted
        );
        trust_path_with_store(&filter_path, &store_path).unwrap();
        assert_eq!(
            check_trust_with_store(&filter_path, &store_path).unwrap(),
            TrustStatus::Trusted
        );

        std::fs::write(&filter_path, "[[filters]]\nmatch_command = \"y\"\n").unwrap();
        assert!(matches!(
            check_trust_with_store(&filter_path, &store_path).unwrap(),
            TrustStatus::ContentChanged { .. }
        ));
    }
}
