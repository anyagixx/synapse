// MODULE_CONTRACT
// MODULE_ID: M-HOOKS
// PURPOSE: Git pre-commit hook manager for staged MyGRACE verification before commits.
// SCOPE: Pre-commit script template, install/status/uninstall lifecycle, foreign hook backup, backup restore, executable permissions, and git repository validation.
// DEPENDS: M-HOOKS, M-GRACE-VERIFY
// LINKS:
//   -> Phase-90 (implements) - git pre-commit verification hook
//   -> M-GRACE-VERIFY (depends) - staged verification command
//   -> NFR-002 (traces_to) - reliable release verification commands

// START_MODULE_MAP
// GitHookReport - Serializable report for install and uninstall actions
// GitHookStatus - Serializable status for active, missing, or foreign hooks
// HookManager::install_git_pre_commit - Installs the Synapse pre-commit hook
// HookManager::uninstall_git_pre_commit - Removes the Synapse hook and restores a backup
// HookManager::git_pre_commit_status - Reports git pre-commit hook state
// install_git_pre_commit_at - Root-scoped install implementation
// uninstall_git_pre_commit_at - Root-scoped uninstall implementation
// git_pre_commit_status_at - Root-scoped status implementation
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Added Phase-90 git pre-commit verification hook lifecycle]
// END_CHANGE_SUMMARY

use super::HookManager;
use serde::Serialize;
use std::path::{Path, PathBuf};

const PRE_COMMIT_FILE: &str = "pre-commit";
const PRE_COMMIT_BACKUP_PREFIX: &str = "pre-commit.synapse-backup";
const SYNAPSE_PRE_COMMIT_MARKER: &str = "Synapse pre-commit verification hook";
const PRE_COMMIT_HOOK: &str = r#"#!/usr/bin/env bash
# Synapse pre-commit verification hook
# Installed by: syn hook install

set -uo pipefail

echo "Synapse: verifying staged changes..."

STAGED="$(git diff --cached --name-only --diff-filter=ACM)"

if [ -z "$STAGED" ]; then
    echo "Synapse: no files staged, skipping verification."
    exit 0
fi

if ! printf '%s\n' "$STAGED" | grep -qE '\.(rs|md|toml|xml)$'; then
    echo "Synapse: no tracked source/docs files staged, skipping verification."
    exit 0
fi

if ! command -v syn >/dev/null 2>&1; then
    echo "Synapse: 'syn' not found, skipping pre-commit verification. Use --no-verify to bypass manually."
    exit 0
fi

if syn verify --staged; then
    echo "Synapse: verification passed."
    exit 0
fi

echo ""
echo "Synapse: verification failed. Fix issues or use --no-verify to bypass."
echo "Run 'syn verify' for details."
exit 1
"#;

// START_public_api

// START_GitHookReport
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GitHookReport {
    pub hook: String,
    pub action: String,
    pub path: String,
    pub backup_path: Option<String>,
}
// END_GitHookReport

// START_GitHookStatus
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum GitHookStatus {
    Active { path: String },
    NotInstalled { path: String },
    Foreign { path: String, message: String },
}
// END_GitHookStatus

impl HookManager {
    // START_CONTRACT_HookManager::install_git_pre_commit
    // PURPOSE: Install the Synapse git pre-commit hook in the current repository.
    // OUTPUTS: { anyhow::Result<GitHookReport> }
    // SIDE_EFFECTS: writes .git/hooks/pre-commit, backs up foreign hooks, and marks the hook executable on Unix
    // LINKS:
    //   -> Phase-90 (implements) - pre-commit hook install
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-HOOKS (verified_by) - git pre-commit lifecycle tests
    // START_hookmanager_install_git_pre_commit
    pub fn install_git_pre_commit(&self) -> anyhow::Result<GitHookReport> {
        let root = std::env::current_dir()?;
        install_git_pre_commit_at(&root)
    }
    // END_hookmanager_install_git_pre_commit

    // START_CONTRACT_HookManager::uninstall_git_pre_commit
    // PURPOSE: Remove the Synapse git pre-commit hook from the current repository and restore a backup when present.
    // OUTPUTS: { anyhow::Result<GitHookReport> }
    // SIDE_EFFECTS: removes .git/hooks/pre-commit and may rename a backup hook into place
    // LINKS:
    //   -> Phase-90 (implements) - pre-commit hook uninstall
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-HOOKS (verified_by) - backup restore tests
    // START_hookmanager_uninstall_git_pre_commit
    pub fn uninstall_git_pre_commit(&self) -> anyhow::Result<GitHookReport> {
        let root = std::env::current_dir()?;
        uninstall_git_pre_commit_at(&root)
    }
    // END_hookmanager_uninstall_git_pre_commit

    // START_CONTRACT_HookManager::git_pre_commit_status
    // PURPOSE: Report whether the current repository has an active, missing, or foreign pre-commit hook.
    // OUTPUTS: { anyhow::Result<GitHookStatus> }
    // LINKS:
    //   -> Phase-90 (implements) - pre-commit hook status
    //   -> NFR-002 (traces_to) - reliable release verification commands
    //   <- V-M-HOOKS (verified_by) - status tests
    // START_hookmanager_git_pre_commit_status
    pub fn git_pre_commit_status(&self) -> anyhow::Result<GitHookStatus> {
        let root = std::env::current_dir()?;
        git_pre_commit_status_at(&root)
    }
    // END_hookmanager_git_pre_commit_status
}

// START_CONTRACT_install_git_pre_commit_at
// PURPOSE: Install or update the Synapse pre-commit hook under an explicit repository root.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<GitHookReport> }
// SIDE_EFFECTS: creates hooks directory, writes hook file, and backs up foreign hooks
// LINKS:
//   -> Phase-90 (implements) - pre-commit hook install
//   -> NFR-002 (traces_to) - reliable release verification commands
//   <- V-M-HOOKS (verified_by) - install and backup tests
// START_install_git_pre_commit_at
fn install_git_pre_commit_at(root: &Path) -> anyhow::Result<GitHookReport> {
    let hooks_dir = git_hooks_dir(root)?;
    std::fs::create_dir_all(&hooks_dir)?;
    let hook_path = hooks_dir.join(PRE_COMMIT_FILE);
    let mut backup_path = None;
    let mut action = "installed".to_string();

    if hook_path.exists() {
        let existing = std::fs::read_to_string(&hook_path)?;
        if is_synapse_pre_commit_hook(&existing) {
            if existing == PRE_COMMIT_HOOK {
                action = "already_installed".to_string();
            } else {
                std::fs::write(&hook_path, PRE_COMMIT_HOOK)?;
                action = "updated".to_string();
            }
        } else {
            let backup = next_backup_path(&hooks_dir);
            std::fs::copy(&hook_path, &backup)?;
            std::fs::write(&hook_path, PRE_COMMIT_HOOK)?;
            set_executable(&hook_path)?;
            backup_path = Some(backup.display().to_string());
            action = "backed_up_and_replaced".to_string();
        }
    } else {
        std::fs::write(&hook_path, PRE_COMMIT_HOOK)?;
    }

    set_executable(&hook_path)?;
    Ok(GitHookReport {
        hook: PRE_COMMIT_FILE.to_string(),
        action,
        path: hook_path.display().to_string(),
        backup_path,
    })
}
// END_install_git_pre_commit_at

// START_CONTRACT_uninstall_git_pre_commit_at
// PURPOSE: Remove a Synapse-owned pre-commit hook and restore the newest Synapse backup if one exists.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<GitHookReport> }
// SIDE_EFFECTS: removes or renames files inside .git/hooks
// LINKS:
//   -> Phase-90 (implements) - pre-commit hook uninstall
//   -> NFR-002 (traces_to) - reliable release verification commands
//   <- V-M-HOOKS (verified_by) - uninstall and restore tests
// START_uninstall_git_pre_commit_at
fn uninstall_git_pre_commit_at(root: &Path) -> anyhow::Result<GitHookReport> {
    let hooks_dir = git_hooks_dir(root)?;
    let hook_path = hooks_dir.join(PRE_COMMIT_FILE);
    if !hook_path.exists() {
        return Ok(GitHookReport {
            hook: PRE_COMMIT_FILE.to_string(),
            action: "not_installed".to_string(),
            path: hook_path.display().to_string(),
            backup_path: None,
        });
    }

    let existing = std::fs::read_to_string(&hook_path)?;
    if !is_synapse_pre_commit_hook(&existing) {
        anyhow::bail!(
            "existing pre-commit hook was not installed by Synapse; remove it manually or run install to back it up"
        );
    }

    std::fs::remove_file(&hook_path)?;
    let backup_path = newest_backup_path(&hooks_dir);
    let action = if let Some(backup) = backup_path.as_ref() {
        std::fs::rename(backup, &hook_path)?;
        set_executable(&hook_path)?;
        "restored_backup"
    } else {
        "uninstalled"
    };

    Ok(GitHookReport {
        hook: PRE_COMMIT_FILE.to_string(),
        action: action.to_string(),
        path: hook_path.display().to_string(),
        backup_path: backup_path.map(|path| path.display().to_string()),
    })
}
// END_uninstall_git_pre_commit_at

// START_CONTRACT_git_pre_commit_status_at
// PURPOSE: Return active, missing, or foreign state for an explicit repository root.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<GitHookStatus> }
// LINKS:
//   -> Phase-90 (implements) - pre-commit hook status
//   -> NFR-002 (traces_to) - reliable release verification commands
//   <- V-M-HOOKS (verified_by) - status tests
// START_git_pre_commit_status_at
fn git_pre_commit_status_at(root: &Path) -> anyhow::Result<GitHookStatus> {
    let hook_path = git_hooks_dir(root)?.join(PRE_COMMIT_FILE);
    let path = hook_path.display().to_string();
    if !hook_path.exists() {
        return Ok(GitHookStatus::NotInstalled { path });
    }

    let existing = std::fs::read_to_string(&hook_path)?;
    if is_synapse_pre_commit_hook(&existing) {
        Ok(GitHookStatus::Active { path })
    } else {
        Ok(GitHookStatus::Foreign {
            path,
            message: "A pre-commit hook exists but was not installed by Synapse.".to_string(),
        })
    }
}
// END_git_pre_commit_status_at

// END_public_api

// START_CONTRACT_git_hooks_dir
// PURPOSE: Resolve the git hooks directory for a repository root.
// INPUTS: { root: &Path }
// OUTPUTS: { anyhow::Result<PathBuf> }
// LINKS:
//   -> Phase-90 (implements) - repository-aware hook install
//   <- V-M-HOOKS (verified_by) - temporary repository tests
// START_git_hooks_dir
fn git_hooks_dir(root: &Path) -> anyhow::Result<PathBuf> {
    let git_dir = root.join(".git");
    if git_dir.is_dir() {
        return Ok(git_dir.join("hooks"));
    }

    let output = std::process::Command::new("git")
        .args(["rev-parse", "--git-common-dir"])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        anyhow::bail!("not a git repository; run git init first");
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        anyhow::bail!("git did not report a hooks directory");
    }
    let common_dir = PathBuf::from(path);
    if common_dir.is_absolute() {
        Ok(common_dir.join("hooks"))
    } else {
        Ok(root.join(common_dir).join("hooks"))
    }
}
// END_git_hooks_dir

// START_CONTRACT_is_synapse_pre_commit_hook
// PURPOSE: Detect Synapse-owned pre-commit hook content by trusted marker.
// INPUTS: { content: &str }
// OUTPUTS: { bool }
// LINKS:
//   -> Phase-90 (implements) - trusted hook ownership checks
//   <- V-M-HOOKS (verified_by) - foreign hook tests
// START_is_synapse_pre_commit_hook
fn is_synapse_pre_commit_hook(content: &str) -> bool {
    content.contains(SYNAPSE_PRE_COMMIT_MARKER) && content.contains("syn verify --staged")
}
// END_is_synapse_pre_commit_hook

// START_CONTRACT_next_backup_path
// PURPOSE: Return a non-conflicting backup path for a foreign pre-commit hook.
// INPUTS: { hooks_dir: &Path }
// OUTPUTS: { PathBuf }
// LINKS:
//   -> Phase-90 (implements) - foreign hook backup
//   <- V-M-HOOKS (verified_by) - backup preservation tests
// START_next_backup_path
fn next_backup_path(hooks_dir: &Path) -> PathBuf {
    let first = hooks_dir.join(PRE_COMMIT_BACKUP_PREFIX);
    if !first.exists() {
        return first;
    }
    for index in 1..1000 {
        let candidate = hooks_dir.join(format!("{PRE_COMMIT_BACKUP_PREFIX}.{index}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    hooks_dir.join(format!("{PRE_COMMIT_BACKUP_PREFIX}.overflow"))
}
// END_next_backup_path

// START_CONTRACT_newest_backup_path
// PURPOSE: Return the newest available Synapse pre-commit backup path.
// INPUTS: { hooks_dir: &Path }
// OUTPUTS: { Option<PathBuf> }
// LINKS:
//   -> Phase-90 (implements) - backup restore
//   <- V-M-HOOKS (verified_by) - backup restore tests
// START_newest_backup_path
fn newest_backup_path(hooks_dir: &Path) -> Option<PathBuf> {
    for index in (1..1000).rev() {
        let candidate = hooks_dir.join(format!("{PRE_COMMIT_BACKUP_PREFIX}.{index}"));
        if candidate.exists() {
            return Some(candidate);
        }
    }
    let first = hooks_dir.join(PRE_COMMIT_BACKUP_PREFIX);
    first.exists().then_some(first)
}
// END_newest_backup_path

// START_CONTRACT_set_executable
// PURPOSE: Mark the hook executable on Unix and leave permissions unchanged elsewhere.
// INPUTS: { path: &Path }
// OUTPUTS: { anyhow::Result<()> }
// SIDE_EFFECTS: updates file permissions on Unix
// LINKS:
//   -> Phase-90 (implements) - executable pre-commit hook
//   <- V-M-HOOKS (verified_by) - executable permission tests
// START_set_executable
fn set_executable(path: &Path) -> anyhow::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}
// END_set_executable

#[cfg(test)]
mod tests {
    use super::*;

    fn init_repo(root: &Path) -> anyhow::Result<()> {
        let output = std::process::Command::new("git")
            .arg("init")
            .current_dir(root)
            .output()?;
        if !output.status.success() {
            anyhow::bail!("git init failed");
        }
        Ok(())
    }

    #[test]
    fn git_pre_commit_install_status_uninstall_round_trip() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        init_repo(temp.path())?;

        let report = install_git_pre_commit_at(temp.path())?;
        assert_eq!(report.action, "installed");
        let hook_path = temp.path().join(".git/hooks/pre-commit");
        assert!(hook_path.exists());
        let content = std::fs::read_to_string(&hook_path)?;
        assert!(is_synapse_pre_commit_hook(&content));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&hook_path)?.permissions().mode();
            assert_eq!(mode & 0o111, 0o111);
        }

        assert!(matches!(
            git_pre_commit_status_at(temp.path())?,
            GitHookStatus::Active { .. }
        ));

        let report = uninstall_git_pre_commit_at(temp.path())?;
        assert_eq!(report.action, "uninstalled");
        assert!(matches!(
            git_pre_commit_status_at(temp.path())?,
            GitHookStatus::NotInstalled { .. }
        ));
        Ok(())
    }

    #[test]
    fn git_pre_commit_install_backs_up_foreign_hook_and_uninstall_restores() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        init_repo(temp.path())?;
        let hook_path = temp.path().join(".git/hooks/pre-commit");
        std::fs::write(&hook_path, "#!/bin/sh\necho foreign\n")?;

        let install = install_git_pre_commit_at(temp.path())?;
        assert_eq!(install.action, "backed_up_and_replaced");
        let backup = install
            .backup_path
            .as_ref()
            .map(PathBuf::from)
            .ok_or_else(|| anyhow::anyhow!("missing backup path"))?;
        assert!(backup.exists());
        assert!(is_synapse_pre_commit_hook(&std::fs::read_to_string(
            &hook_path
        )?));

        let uninstall = uninstall_git_pre_commit_at(temp.path())?;
        assert_eq!(uninstall.action, "restored_backup");
        assert_eq!(
            std::fs::read_to_string(&hook_path)?,
            "#!/bin/sh\necho foreign\n"
        );
        assert!(matches!(
            git_pre_commit_status_at(temp.path())?,
            GitHookStatus::Foreign { .. }
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn git_pre_commit_blocks_failure_and_allows_no_verify_bypass() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;

        let temp = tempfile::tempdir()?;
        init_repo(temp.path())?;
        install_git_pre_commit_at(temp.path())?;
        std::fs::write(temp.path().join("broken.rs"), "fn broken() {}\n")?;
        let add = std::process::Command::new("git")
            .args(["add", "broken.rs"])
            .current_dir(temp.path())
            .output()?;
        if !add.status.success() {
            anyhow::bail!("git add failed");
        }

        let bin_dir = temp.path().join("bin");
        std::fs::create_dir_all(&bin_dir)?;
        let fake_syn = bin_dir.join("syn");
        std::fs::write(
            &fake_syn,
            "#!/usr/bin/env bash\nif [ \"$1\" = \"verify\" ] && [ \"$2\" = \"--staged\" ]; then exit 1; fi\nexit 0\n",
        )?;
        let mut permissions = std::fs::metadata(&fake_syn)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&fake_syn, permissions)?;
        let path = format!(
            "{}:{}",
            bin_dir.display(),
            std::env::var("PATH").unwrap_or_default()
        );

        let blocked = std::process::Command::new("git")
            .args([
                "-c",
                "user.email=synapse@example.invalid",
                "-c",
                "user.name=Synapse Test",
                "commit",
                "-m",
                "blocked",
            ])
            .env("PATH", &path)
            .current_dir(temp.path())
            .output()?;
        assert!(!blocked.status.success());

        let bypassed = std::process::Command::new("git")
            .args([
                "-c",
                "user.email=synapse@example.invalid",
                "-c",
                "user.name=Synapse Test",
                "commit",
                "--no-verify",
                "-m",
                "bypassed",
            ])
            .env("PATH", &path)
            .current_dir(temp.path())
            .output()?;
        assert!(bypassed.status.success());
        Ok(())
    }
}
