// MODULE_CONTRACT
// MODULE_ID: M-TESTS-PARITY
// PURPOSE: Ensure README, docs, install scripts, release workflow, and code claims match product capabilities
// SCOPE: Compare README tool count, README command count, verify check count, CLI flag truth including route/session/adapter/rewrite/filter/local RTK adapter/discover/learn/hooks-audit/cc-economics flags, public docs, install docs, Linux/macOS release matrix, checksum integrity, release freshness, installer source fallback, and release smoke coverage
// DEPENDS: M-CAPABILITIES, M-CLI, M-INSTALL, M-CI, M-CI-RELEASE-SMOKE
// LINKS: docs/phases/Phase-27.xml

// START_MODULE_MAP
// test_mcp_tool_count_matches_capabilities — MCP tool count check
// test_command_count_matches_capabilities — Command count check
// test_verify_checks_consistent — Verify check consistency
// test_no_ghost_commands — No ghost commands, unsupported flags, or obsolete public URLs
// test_package_metadata_preserves_syn_binary_without_crates_conflict — Package metadata check
// test_install_default_version_matches_package — Installer default tag, source-ref fallback, and pre-tag main fallback check
// test_install_docs_use_supported_url — Install documentation URL check
// test_release_tag_version_guard_is_enforced — Release tag/version guard check
// test_release_freshness_guard_blocks_stale_tags — Isolated stale tag freshness guard check
// test_release_artifact_matches_installer — Release artifact naming check
// test_platform_claims_match_release_truth — Platform support truth check
// test_installer_uses_safe_temp_dir_and_cleanup — Installer temp safety check
// test_release_smoke_builds_package_and_runs_binary — Release smoke behavior check
// test_release_checksum_integrity_is_enforced — Release checksum integrity check
// test_release_policy_gates_are_explicit — Release notes and audit policy check
// test_release_matrix_declares_linux_macos_targets — Linux/macOS release matrix check
// test_ci_declares_fresh_install_evidence — Hosted CI fresh install evidence check
// test_mcp_help_hides_unimplemented_flags — MCP CLI truth check
// test_installer_dry_run_maps_linux_macos_artifacts — Installer dry-run mapping check
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v5.4.0 - Advanced public install URL expectation to v2.6.2]
// END_CHANGE_SUMMARY

use syn::capabilities;

const INSTALL_SCRIPT: &str = include_str!("../install.sh");
const CARGO_TOML: &str = include_str!("../Cargo.toml");
const CI_SCRIPT: &str = include_str!("../scripts/ci.sh");
const RELEASE_FRESHNESS_GUARD: &str = include_str!("../scripts/release_freshness_guard.sh");
const RELEASE_SMOKE_SCRIPT: &str = include_str!("../scripts/release_install_smoke.sh");
const CI_WORKFLOW: &str = include_str!("../.github/workflows/ci.yml");
const RELEASE_WORKFLOW: &str = include_str!("../.github/workflows/release.yml");
const RELEASE_VERSION_GUARD: &str = include_str!("../scripts/release_version_guard.sh");
const README: &str = include_str!("../README.md");
const INSTALL_DOC: &str = include_str!("../INSTALL.md");
const COMMANDS_DOC: &str = include_str!("../docs/COMMANDS.md");
const QUICKSTART: &str = include_str!("../docs/QUICKSTART.md");
const FAQ: &str = include_str!("../docs/FAQ.md");
const WORKFLOW_DOC: &str = include_str!("../docs/WORKFLOW.md");
const SUPPORT_DOC: &str = include_str!("../docs/SUPPORT.md");
const INSTALL_COMMAND: &str =
    "curl -fsSL https://raw.githubusercontent.com/anyagixx/synapse/v2.6.2/install.sh | sh";
const EXPECTED_PREBUILT_ARTIFACTS: [&str; 4] = [
    "syn-x86_64-unknown-linux-gnu.tar.gz",
    "syn-aarch64-unknown-linux-gnu.tar.gz",
    "syn-x86_64-apple-darwin.tar.gz",
    "syn-aarch64-apple-darwin.tar.gz",
];
// START_CONTRACT_public_docs
// PURPOSE: Return public docs whose shipped-command and install claims must stay truthful
// OUTPUTS: { Vec<(&str, &str)> — doc display names and contents }
fn public_docs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("README.md", README),
        ("INSTALL.md", INSTALL_DOC),
        ("docs/COMMANDS.md", COMMANDS_DOC),
        ("docs/QUICKSTART.md", QUICKSTART),
        ("docs/FAQ.md", FAQ),
        ("docs/WORKFLOW.md", WORKFLOW_DOC),
        ("docs/SUPPORT.md", SUPPORT_DOC),
    ]
}

// START_CONTRACT_documented_syn_invocations
// PURPOSE: Extract documented `syn ...` invocations from code spans and shell snippets
// INPUTS: { doc: &str — markdown document content }
// OUTPUTS: { Vec<Vec<String>> — tokens following syn for each invocation }
fn documented_syn_invocations(doc: &str) -> Vec<Vec<String>> {
    let mut invocations = Vec::new();
    for line in doc.lines() {
        let trimmed = line.trim_start().trim_start_matches("$ ").trim_start();
        if let Some(rest) = trimmed.strip_prefix("syn ") {
            let tokens = invocation_tokens(rest);
            if !tokens.is_empty() {
                invocations.push(tokens);
            }
        }
    }
    for (idx, segment) in doc.split('`').enumerate() {
        if idx % 2 == 1 {
            let trimmed = segment.trim();
            if let Some(rest) = trimmed.strip_prefix("syn ") {
                let tokens = invocation_tokens(rest);
                if !tokens.is_empty() {
                    invocations.push(tokens);
                }
            }
        }
    }
    invocations
}

// START_CONTRACT_invocation_tokens
// PURPOSE: Normalize one documented syn invocation into comparable tokens
// INPUTS: { rest: &str — text after `syn ` }
// OUTPUTS: { Vec<String> — trimmed invocation tokens }
fn invocation_tokens(rest: &str) -> Vec<String> {
    rest.split_whitespace()
        .map(|token| {
            token
                .trim_matches(|c: char| "`|,.;:)".contains(c))
                .to_string()
        })
        .filter(|token| !token.is_empty())
        .collect()
}

// START_CONTRACT_allowed_flags_for_command
// PURPOSE: Return supported documented flags for a CLI command
// INPUTS: { command: &str — command token after syn }
// OUTPUTS: { &'static [&'static str] — allowed flags }
fn allowed_flags_for_command(command: &str) -> &'static [&'static str] {
    match command {
        "gain" => &["--graph", "--sessions", "--adapters"],
        "index" => &["--watch", "--no-git"],
        "doctor" => &["--deps"],
        "discover" => &["--json", "--limit"],
        "env" => &["--filter", "--show-all"],
        "filters" => &["--filter", "--require-all"],
        "hook" => &["--agent"],
        "hooks" => &["--json"],
        "json" => &["--depth", "--keys-only"],
        "learn" => &["--json"],
        "session" => &["--json"],
        "cc-economics" => &[
            "--daily",
            "--weekly",
            "--monthly",
            "--all",
            "--format",
            "-d",
            "-w",
            "-m",
            "-a",
            "-f",
        ],
        "pipe" => &["--filter"],
        "proxy" => &["--", "--route"],
        "refresh" => &["--fix"],
        "review" => &["--mode", "--profile"],
        "rtk-parity" => &["--source", "--json", "--ci", "--full"],
        "verify" => &["--profile"],
        _ => &[],
    }
}

#[test]
// START_CONTRACT_test_mcp_tool_count_matches_capabilities
// PURPOSE: Verify MCP capability counts and uniqueness
fn test_mcp_tool_count_matches_capabilities() {
    let expected = capabilities::MCP_TOOLS.len();
    assert_eq!(expected, capabilities::TOTAL_MCP_TOOL_COUNT);
    assert_eq!(capabilities::CORE_MCP_TOOL_COUNT, 23);
    assert_eq!(capabilities::GRACE_SKILL_TOOL_COUNT, 16);
    assert_eq!(capabilities::skill_defs_count(), 16);
    assert!(!capabilities::MCP_TOOLS.is_empty(), "MCP tools list empty");
    let names: Vec<&str> = capabilities::MCP_TOOLS.iter().map(|(n, _)| *n).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(names.len(), unique.len(), "Duplicate MCP tool names found");
}

#[test]
// START_CONTRACT_test_command_count_matches_capabilities
// PURPOSE: Verify CLI command registry has enough unique commands
fn test_command_count_matches_capabilities() {
    assert!(
        capabilities::COMMANDS.len() >= 15,
        "Expected at least 15 commands"
    );
    let names: Vec<&str> = capabilities::COMMANDS.iter().map(|(n, _)| *n).collect();
    let unique: std::collections::HashSet<_> = names.iter().collect();
    assert_eq!(
        names.len(),
        unique.len(),
        "Duplicate command names in registry"
    );
}

#[test]
// START_CONTRACT_test_verify_checks_consistent
// PURPOSE: Verify verification check registry exposes expected checks
fn test_verify_checks_consistent() {
    assert!(
        capabilities::VERIFY_CHECKS.len() >= 8,
        "Expected at least 8 verify checks"
    );
}

#[test]
// START_CONTRACT_test_no_ghost_commands
// PURPOSE: Verify public docs only mention shipped commands and supported install surfaces
fn test_no_ghost_commands() {
    let shipped: std::collections::HashSet<_> = capabilities::COMMANDS
        .iter()
        .map(|(name, _)| *name)
        .collect();
    let allowed_global_flags = ["--help", "--version", "-h", "-V"];
    let forbidden = [
        "syn plan",
        "syn execute",
        "syn fix",
        "syn explain",
        "syn mcp --http",
        "syn mcp-proxy",
        "syn logs",
        "syn telemetry",
        "syn completion",
        "syn --quiet",
        "syn --verbose",
        "syn --json",
        "https://synapse.dev/install.sh",
        "github.com/synapse-ai",
        "syn-x86_64-pc-windows-msvc",
        "unknown-linux-musl",
        "brew install synapse-ai",
    ];

    for (name, desc) in capabilities::COMMANDS {
        assert!(!desc.is_empty(), "Command '{}' has empty description", name);
        assert!(!name.is_empty(), "Empty command name");
    }
    for (doc_name, doc) in public_docs() {
        for invocation in documented_syn_invocations(doc) {
            let token = &invocation[0];
            if token.starts_with('-') {
                assert!(
                    allowed_global_flags.contains(&token.as_str()),
                    "{} documents unsupported global flag syn {}",
                    doc_name,
                    token
                );
            } else {
                assert!(
                    shipped.contains(token.as_str()),
                    "{} documents unsupported command syn {}",
                    doc_name,
                    token
                );
            }
            if !token.starts_with('-') {
                let allowed_flags = allowed_flags_for_command(token);
                for arg in invocation.iter().skip(1).filter(|arg| arg.starts_with('-')) {
                    assert!(
                        allowed_flags.contains(&arg.as_str()),
                        "{} documents unsupported flag syn {} {}",
                        doc_name,
                        token,
                        arg
                    );
                }
            }
        }
        for snippet in forbidden {
            assert!(
                !doc.contains(snippet),
                "{} must not document unsupported or obsolete surface: {}",
                doc_name,
                snippet
            );
        }
    }
}

#[test]
// START_CONTRACT_test_package_metadata_preserves_syn_binary_without_crates_conflict
// PURPOSE: Verify Cargo package metadata avoids crates.io `syn` conflict while preserving the installed binary name
fn test_package_metadata_preserves_syn_binary_without_crates_conflict() {
    assert!(
        CARGO_TOML.contains("name = \"synapse-agent\""),
        "Cargo package name must avoid crates.io syn parser conflict"
    );
    assert!(
        CARGO_TOML.contains("[lib]\nname = \"syn\"")
            && CARGO_TOML.contains("[[bin]]\nname = \"syn\""),
        "library crate and installed binary must remain named syn"
    );
    assert!(
        CARGO_TOML.contains("homepage = \"https://github.com/anyagixx/synapse\""),
        "package homepage must not point at parked domains"
    );
}

#[test]
// START_CONTRACT_test_install_default_version_matches_package
// PURPOSE: Verify install.sh defaults to the current Cargo package tag
fn test_install_default_version_matches_package() {
    let expected_default = format!("DEFAULT_VERSION=\"v{}\"", env!("CARGO_PKG_VERSION"));
    assert!(
        INSTALL_SCRIPT.contains(&expected_default),
        "install.sh must default to {}",
        expected_default
    );
    for marker in [
        "cargo install --locked --git https://github.com/anyagixx/synapse --tag \"${VERSION}\"",
        "cargo install --locked --git https://github.com/anyagixx/synapse --rev \"${SYN_INSTALL_SOURCE_REF}\"",
        "cargo install --locked --git https://github.com/anyagixx/synapse --branch main",
        "remote_tag_available \"$VERSION\"",
    ] {
        assert!(
            INSTALL_SCRIPT.contains(marker),
            "install.sh must keep locked source fallback marker {marker}"
        );
    }
}

#[test]
// START_CONTRACT_test_install_docs_use_supported_url
// PURPOSE: Verify install docs use the repository-hosted installer URL
fn test_install_docs_use_supported_url() {
    for (name, doc) in [
        ("README.md", README),
        ("INSTALL.md", INSTALL_DOC),
        ("docs/QUICKSTART.md", QUICKSTART),
        ("docs/FAQ.md", FAQ),
    ] {
        assert!(
            doc.contains(INSTALL_COMMAND),
            "{} must document the supported install command",
            name
        );
        assert!(
            !doc.contains("https://synapse.dev/install.sh"),
            "{} must not document unsupported hosted installer URLs",
            name
        );
    }
}

#[test]
// START_CONTRACT_test_release_tag_version_guard_is_enforced
// PURPOSE: Verify release workflow rejects tags that differ from Cargo.toml package version
fn test_release_tag_version_guard_is_enforced() {
    assert!(
        RELEASE_WORKFLOW.contains("bash scripts/release_version_guard.sh"),
        "release workflow must run the release version guard"
    );
    assert!(
        RELEASE_VERSION_GUARD.contains("expected_tag=\"v${cargo_version}\"")
            && RELEASE_VERSION_GUARD.contains("SYN_RELEASE_TAG"),
        "release version guard must compare selected tag to Cargo.toml version"
    );
}

#[test]
// START_CONTRACT_test_release_freshness_guard_blocks_stale_tags
// PURPOSE: Verify release gates detect when Cargo version points to a stale already-published tag
fn test_release_freshness_guard_blocks_stale_tags() {
    for marker in [
        "release_freshness_guard.sh",
        "[CI][run_ci_gate][RELEASE_FRESHNESS]",
        "[CI][release_freshness_guard][PASS]",
        "Bump Cargo.toml/install.sh version before release work continues.",
    ] {
        assert!(
            CI_SCRIPT.contains(marker) || RELEASE_FRESHNESS_GUARD.contains(marker),
            "release freshness guard marker missing: {marker}"
        );
    }
    let repo = tempfile::tempdir().expect("temp repo");
    std::fs::create_dir(repo.path().join("scripts")).expect("scripts dir");
    std::fs::write(
        repo.path().join("Cargo.toml"),
        "[package]\nname = \"freshness-test\"\nversion = \"9.9.9\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(
        repo.path().join("scripts/release_freshness_guard.sh"),
        RELEASE_FRESHNESS_GUARD,
    )
    .expect("write guard");
    for args in [
        vec!["init"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Synapse Test",
            "-c",
            "user.email=synapse@example.invalid",
            "commit",
            "-m",
            "initial",
        ],
        vec!["tag", "v9.9.9"],
    ] {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .output()
            .expect("git should execute");
        assert!(
            output.status.success(),
            "git setup failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    std::fs::write(repo.path().join("README.md"), "post-tag work\n").expect("write readme");
    for args in [
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Synapse Test",
            "-c",
            "user.email=synapse@example.invalid",
            "commit",
            "-m",
            "post tag",
        ],
    ] {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(repo.path())
            .output()
            .expect("git should execute");
        assert!(
            output.status.success(),
            "git post-tag setup failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = std::process::Command::new("bash")
        .arg("scripts/release_freshness_guard.sh")
        .current_dir(repo.path())
        .output()
        .expect("release freshness guard should execute");
    assert!(
        !output.status.success(),
        "release freshness guard should reject stale tags\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[CI][release_freshness_guard][FAIL]")
            && stdout.contains("Bump Cargo.toml/install.sh version before release work continues."),
        "freshness guard should report stale tag remediation: {}",
        stdout
    );
}

#[test]
// START_CONTRACT_test_release_artifact_matches_installer
// PURPOSE: Verify release workflow publishes the Linux artifact expected by install.sh
fn test_release_artifact_matches_installer() {
    assert!(
        INSTALL_SCRIPT.contains("TARBALL=\"syn-${ARCH}-${OS}.tar.gz\""),
        "install.sh must derive tarball names from detected arch and OS"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT.contains("tar -czf \"$artifact_path\" -C \"$dist_dir\" ."),
        "release smoke must package tarballs through the shared artifact path"
    );
    for artifact in EXPECTED_PREBUILT_ARTIFACTS {
        assert!(
            RELEASE_WORKFLOW.contains(artifact),
            "release workflow must declare {}",
            artifact
        );
    }
    assert!(
        RELEASE_SMOKE_SCRIPT
            .contains("cargo build --release --locked --target \"$release_target\""),
        "release smoke must use the locked dependency graph and explicit target"
    );
    assert!(
        RELEASE_WORKFLOW.contains("SHA256SUMS"),
        "release workflow must publish release checksums"
    );
}

#[test]
// START_CONTRACT_test_platform_claims_match_release_truth
// PURPOSE: Verify FAQ platform support claims distinguish prebuilt artifacts from source fallback
fn test_platform_claims_match_release_truth() {
    assert!(
        FAQ.contains("Prebuilt release artifacts:"),
        "FAQ must introduce the Linux/macOS prebuilt matrix"
    );
    for platform in [
        "Linux x86_64",
        "Linux aarch64",
        "macOS x86_64",
        "macOS arm64",
    ] {
        assert!(
            FAQ.contains(platform),
            "FAQ must state {} support",
            platform
        );
    }
    assert!(
        FAQ.contains("Windows packaging is planned later and is not part of the current Linux/macOS release matrix."),
        "FAQ must state Windows is intentionally deferred"
    );
}

#[test]
// START_CONTRACT_test_installer_uses_safe_temp_dir_and_cleanup
// PURPOSE: Verify install.sh uses a private temporary directory and post-install smoke
fn test_installer_uses_safe_temp_dir_and_cleanup() {
    assert!(
        INSTALL_SCRIPT.contains("mktemp -d \"${TMPDIR:-/tmp}/synapse-install.XXXXXX\""),
        "install.sh must create a private temporary directory"
    );
    assert!(
        INSTALL_SCRIPT.contains("trap cleanup EXIT HUP INT TERM"),
        "install.sh must clean temporary files on exit"
    );
    assert!(
        INSTALL_SCRIPT.contains("SYN_INSTALL_DIR"),
        "install.sh must support explicit install directory override"
    );
    assert!(
        INSTALL_SCRIPT.contains("\"$INSTALLED_BIN\" --version >/dev/null"),
        "install.sh must smoke-check the installed binary"
    );
    assert!(
        !INSTALL_SCRIPT.contains("/tmp/syn.tar.gz") && !INSTALL_SCRIPT.contains("/tmp/syn "),
        "install.sh must not use fixed /tmp paths for release artifacts"
    );
}

#[test]
// START_CONTRACT_test_release_smoke_builds_package_and_runs_binary
// PURPOSE: Verify CI release smoke builds, packages, extracts, and runs the release binary
fn test_release_smoke_builds_package_and_runs_binary() {
    assert!(
        RELEASE_SMOKE_SCRIPT
            .contains("cargo build --release --locked --target \"$release_target\""),
        "release smoke must build with locked dependencies"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT
            .contains("artifact_name=\"${SYN_RELEASE_ARTIFACT:-syn-${release_target}.tar.gz}\""),
        "release smoke must derive the installer-facing artifact name"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT.contains("tar -czf \"$artifact_path\" -C \"$dist_dir\" ."),
        "release smoke must create the release tarball"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT.contains("tar -xzf \"$artifact_path\" -C \"$unpack_dir\""),
        "release smoke must extract the release tarball"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT.contains("\"$unpack_dir/syn\" --version"),
        "release smoke must execute the packaged binary"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT.contains("expected_version=")
            && RELEASE_SMOKE_SCRIPT.contains("version_output=")
            && RELEASE_SMOKE_SCRIPT.contains("syn ${expected_version}"),
        "release smoke must assert packaged syn --version matches Cargo.toml"
    );
    assert!(
        CI_SCRIPT.contains("bash scripts/release_install_smoke.sh")
            && CI_WORKFLOW.contains("bash scripts/release_install_smoke.sh"),
        "local and hosted CI must run the release/install smoke gate"
    );
}

#[test]
// START_CONTRACT_test_release_checksum_integrity_is_enforced
// PURPOSE: Verify release workflow, installer, and smoke gate enforce SHA256 checksum integrity
fn test_release_checksum_integrity_is_enforced() {
    assert!(
        RELEASE_WORKFLOW.contains("sort -k2 > dist/SHA256SUMS")
            && RELEASE_WORKFLOW.contains("sha256sum -c SHA256SUMS"),
        "release workflow must aggregate and verify SHA256SUMS for uploaded artifacts"
    );
    assert!(
        RELEASE_WORKFLOW.contains("SHA256SUMS"),
        "release workflow must upload SHA256SUMS"
    );
    assert!(
        INSTALL_SCRIPT.contains("CHECKSUM_URL=") && INSTALL_SCRIPT.contains("SHA256SUMS"),
        "install.sh must download the release checksum file"
    );
    assert!(
        INSTALL_SCRIPT.contains("verify_release_checksum"),
        "install.sh must verify release checksums before extraction"
    );
    assert!(
        INSTALL_SCRIPT.contains("sha256sum \"$artifact_path\"")
            && INSTALL_SCRIPT.contains("shasum -a 256 \"$artifact_path\""),
        "install.sh must support Linux and macOS SHA256 verification tools"
    );
    assert!(
        RELEASE_SMOKE_SCRIPT.contains("sha256sum \"$artifact_base\" > \"$checksum_base\"")
            && RELEASE_SMOKE_SCRIPT.contains("sha256sum -c \"$checksum_base\""),
        "release smoke must generate and verify SHA256SUMS"
    );
}

#[test]
// START_CONTRACT_test_release_policy_gates_are_explicit
// PURPOSE: Verify GitHub releases get notes and CI treats cargo audit warnings as failures
fn test_release_policy_gates_are_explicit() {
    assert!(
        RELEASE_WORKFLOW.contains("generate_release_notes: true"),
        "release workflow must generate non-empty release notes"
    );
    assert!(
        CI_WORKFLOW.contains("cargo audit --deny warnings"),
        "hosted CI security job must deny cargo audit warnings explicitly"
    );
    assert!(
        !CI_WORKFLOW.contains("cargo audit\n"),
        "hosted CI must not use cargo audit without explicit warning policy"
    );
}

#[test]
// START_CONTRACT_test_release_matrix_declares_linux_macos_targets
// PURPOSE: Verify release and CI workflows declare the Linux/macOS prebuilt artifact matrix
fn test_release_matrix_declares_linux_macos_targets() {
    for target in [
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    ] {
        assert!(
            RELEASE_WORKFLOW.contains(target) && CI_WORKFLOW.contains(target),
            "release and CI workflows must include target {}",
            target
        );
    }
    for runner in [
        "ubuntu-latest",
        "ubuntu-24.04-arm",
        "macos-15-intel",
        "macos-latest",
    ] {
        assert!(
            RELEASE_WORKFLOW.contains(runner) && CI_WORKFLOW.contains(runner),
            "release and CI workflows must include runner {}",
            runner
        );
    }
    assert!(
        !RELEASE_WORKFLOW.contains("windows"),
        "Phase 5 release matrix must not add Windows prebuilt release targets"
    );
}

#[test]
// START_CONTRACT_test_ci_declares_fresh_install_evidence
// PURPOSE: Verify push CI captures hosted public-installer fresh install evidence on Linux/macOS
fn test_ci_declares_fresh_install_evidence() {
    for marker in [
        "fresh-install:",
        "github.event_name == 'push'",
        "bash scripts/fresh_install_smoke.sh",
        "raw.githubusercontent.com/anyagixx/synapse/${{ github.sha }}/install.sh",
        "SYN_INSTALL_SOURCE_REF",
        "ubuntu-latest",
        "macos-latest",
        "Fresh install smoke",
        "Install dir: temporary",
    ] {
        assert!(
            CI_WORKFLOW.contains(marker),
            "CI workflow must declare hosted fresh install marker {marker}"
        );
    }
}

#[test]
// START_CONTRACT_test_mcp_help_hides_unimplemented_flags
// PURPOSE: Verify public MCP help does not expose HTTP or LSP flags before those transports are implemented
fn test_mcp_help_hides_unimplemented_flags() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_syn"))
        .args(["mcp", "--help"])
        .output()
        .expect("syn mcp --help should execute");
    assert!(output.status.success(), "syn mcp --help failed");
    let help = String::from_utf8_lossy(&output.stdout);
    for forbidden in ["--http", "--bind", "--with-lsp"] {
        assert!(
            !help.contains(forbidden),
            "syn mcp help must not expose unimplemented flag {forbidden}: {help}"
        );
    }
}

#[test]
// START_CONTRACT_test_installer_dry_run_maps_linux_macos_artifacts
// PURPOSE: Verify install.sh dry-run maps Linux/macOS uname values to expected artifact names
fn test_installer_dry_run_maps_linux_macos_artifacts() {
    let cases = [
        ("Linux", "x86_64", "syn-x86_64-unknown-linux-gnu.tar.gz"),
        ("Linux", "aarch64", "syn-aarch64-unknown-linux-gnu.tar.gz"),
        ("Darwin", "x86_64", "syn-x86_64-apple-darwin.tar.gz"),
        ("Darwin", "arm64", "syn-aarch64-apple-darwin.tar.gz"),
    ];
    for (system, machine, artifact) in cases {
        let output = std::process::Command::new("sh")
            .arg("install.sh")
            .arg("--dry-run")
            .env("SYN_INSTALL_UNAME_S", system)
            .env("SYN_INSTALL_UNAME_M", machine)
            .output()
            .expect("install.sh dry-run should execute");
        assert!(
            output.status.success(),
            "install.sh dry-run failed for {system}/{machine}"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(&format!("artifact={}", artifact)),
            "dry-run output for {}/{} must contain {}: {}",
            system,
            machine,
            artifact,
            stdout
        );
    }
}
