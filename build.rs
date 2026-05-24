// MODULE_CONTRACT
// MODULE_ID: M-BUILD
// PURPOSE: Cargo build script — declares rebuild triggers for embedded proxy filters and OpenCode assets
// SCOPE: Cargo rerun-if-changed directives for Synapse/RTK built-in filters and OpenCode assets only
// DEPENDS: M-PROXY-FILTER, M-HOOKS
// LINKS: Cargo.toml

// START_MODULE_MAP
// main — Emits cargo rerun-if-changed directives
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.6.0 — Added RTK built-in filter pack rebuild tracking]
// END_CHANGE_SUMMARY

// START_CONTRACT_main
// PURPOSE: Tell Cargo when to rerun the build script
// OUTPUTS: { () }
// SIDE_EFFECTS: prints cargo directives to stdout
// START_main
fn main() {
    println!("cargo::rerun-if-changed=src/proxy/builtin_filters.toml");
    println!("cargo::rerun-if-changed=src/proxy/rtk_builtin_filters.toml");
    println!("cargo::rerun-if-changed=.opencode/");
}
// END_main
