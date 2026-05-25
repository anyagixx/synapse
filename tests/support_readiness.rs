// MODULE_CONTRACT
// MODULE_ID: M-TESTS-PARITY
// PURPOSE: Ensure Linux/macOS install support diagnostics stay truthful and documented.
// SCOPE: Validate installer diagnose mode, unsupported-platform guidance, support docs, and no Windows install claims.
// DEPENDS: M-INSTALL, M-TESTS-PARITY
// LINKS: install.sh, INSTALL.md, docs/FAQ.md, docs/SUPPORT.md

// START_MODULE_MAP
// test_installer_diagnose_reports_supported_matrix - Diagnose mode reports Linux/macOS artifacts
// test_installer_diagnose_reports_os_and_arch_failures - Diagnose mode reports independent platform failures
// test_installer_unsupported_platform_guidance_is_actionable - Unsupported platform failure points to diagnostics
// test_support_docs_document_diagnostics - Public support docs document diagnostic mode
// test_support_docs_do_not_add_windows_install_claims - Support docs keep Windows packaging deferred
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.3.0 - Advanced diagnostic docs to v2.6.2 installer]
// END_CHANGE_SUMMARY

const INSTALL_SCRIPT: &str = include_str!("../install.sh");
const INSTALL_DOC: &str = include_str!("../INSTALL.md");
const FAQ_DOC: &str = include_str!("../docs/FAQ.md");
const SUPPORT_DOC: &str = include_str!("../docs/SUPPORT.md");

#[test]
// START_CONTRACT_test_installer_diagnose_reports_supported_matrix
// PURPOSE: Verify installer diagnose mode reports Linux/macOS artifact mapping and support hints.
fn test_installer_diagnose_reports_supported_matrix() {
    let cases = [
        ("Linux", "x86_64", "syn-x86_64-unknown-linux-gnu.tar.gz"),
        ("Darwin", "arm64", "syn-aarch64-apple-darwin.tar.gz"),
    ];

    for (system, machine, artifact) in cases {
        let output = std::process::Command::new("sh")
            .arg("install.sh")
            .arg("--diagnose")
            .env("SYN_INSTALL_UNAME_S", system)
            .env("SYN_INSTALL_UNAME_M", machine)
            .output()
            .expect("install.sh diagnose should execute");

        assert!(
            output.status.success(),
            "diagnose failed for {system}/{machine}"
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        let markers = vec![
            "mode=diagnose".to_string(),
            "os_status=ok".to_string(),
            "arch_status=ok".to_string(),
            "platform_status=ok".to_string(),
            format!("artifact={artifact}"),
            "install_dir_status=".to_string(),
            "tool.curl=".to_string(),
            "tool.tar=".to_string(),
            "tool.sha256=".to_string(),
            "windows_packaging=deferred".to_string(),
            "support_hint=Use SYN_INSTALL_DIR".to_string(),
        ];
        for marker in markers {
            assert!(
                stdout.contains(&marker),
                "diagnose output missing {marker}: {stdout}"
            );
        }
    }
}

#[test]
// START_CONTRACT_test_installer_diagnose_reports_os_and_arch_failures
// PURPOSE: Verify diagnose mode reports unsupported OS and architecture independently.
fn test_installer_diagnose_reports_os_and_arch_failures() {
    let output = std::process::Command::new("sh")
        .arg("install.sh")
        .arg("--diagnose")
        .env("SYN_INSTALL_UNAME_S", "Windows_NT")
        .env("SYN_INSTALL_UNAME_M", "sparc")
        .output()
        .expect("install.sh diagnose should execute");

    assert!(output.status.success(), "diagnose mode must not fail");
    let stdout = String::from_utf8_lossy(&output.stdout);
    for marker in [
        "os_status=unsupported",
        "arch_status=unsupported",
        "platform_status=unsupported-os-and-arch",
        "artifact=none",
    ] {
        assert!(
            stdout.contains(marker),
            "diagnose output missing {marker}: {stdout}"
        );
    }
}

#[test]
// START_CONTRACT_test_installer_unsupported_platform_guidance_is_actionable
// PURPOSE: Verify unsupported platforms fail with Linux/macOS scope and diagnostic guidance.
fn test_installer_unsupported_platform_guidance_is_actionable() {
    let output = std::process::Command::new("sh")
        .arg("install.sh")
        .arg("--dry-run")
        .env("SYN_INSTALL_UNAME_S", "Windows_NT")
        .env("SYN_INSTALL_UNAME_M", "x86_64")
        .output()
        .expect("install.sh dry-run should execute");

    assert!(!output.status.success(), "unsupported platform must fail");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        combined.contains("Linux and macOS are supported")
            && combined.contains("Windows packaging is planned later")
            && combined.contains("Run: sh install.sh --diagnose"),
        "unsupported platform guidance is not actionable: {combined}"
    );
}

#[test]
// START_CONTRACT_test_support_docs_document_diagnostics
// PURPOSE: Verify user-facing install docs document the diagnostic mode and support report contents.
fn test_support_docs_document_diagnostics() {
    for (name, doc) in [
        ("INSTALL.md", INSTALL_DOC),
        ("docs/FAQ.md", FAQ_DOC),
        ("docs/SUPPORT.md", SUPPORT_DOC),
    ] {
        assert!(
            doc.contains("sh /tmp/synapse-install.sh --diagnose"),
            "{name} must document installer diagnose mode"
        );
        assert!(
            doc.contains("https://raw.githubusercontent.com/anyagixx/synapse/v2.6.2/install.sh"),
            "{name} must use the supported installer URL"
        );
    }
    assert!(
        INSTALL_SCRIPT.contains("--diagnose")
            && INSTALL_SCRIPT.contains("print_diagnostics")
            && INSTALL_SCRIPT.contains("support_hint=Use SYN_INSTALL_DIR"),
        "install.sh must implement diagnostic support mode"
    );
}

#[test]
// START_CONTRACT_test_support_docs_do_not_add_windows_install_claims
// PURPOSE: Verify support docs keep Windows install support deferred until packaging exists.
fn test_support_docs_do_not_add_windows_install_claims() {
    for forbidden in [
        "syn-x86_64-pc-windows-msvc",
        "powershell",
        "winget install",
        "choco install",
    ] {
        assert!(
            !SUPPORT_DOC.contains(forbidden),
            "support docs must not claim Windows install support through {forbidden}"
        );
    }
    assert!(
        SUPPORT_DOC.contains("Windows packaging is planned later"),
        "support docs must state Windows packaging is deferred"
    );
}
