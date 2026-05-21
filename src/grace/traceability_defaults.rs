// MODULE_CONTRACT
// MODULE_ID: M-GRACE-TRACEABILITY
// PURPOSE: Product-intent defaults for Synapse traceability adoption
// SCOPE: TraceLinkSeed and module_trace_defaults mapping from module families to UC/NFR targets
// DEPENDS: M-GRACE-TRACEABILITY
// LINKS:
//   -> V-M-GRACE-TRACEABILITY (verified_by) - traceability defaults are covered by traceability tests
//   -> UC-002 (implements) - verify and review bounded changes with traceable artifacts
//   -> NFR-002 (traces_to) - verification and review must not panic on malformed project state

// START_MODULE_MAP
// TraceLinkSeed - One static product-intent default edge
// module_trace_defaults - Maps Synapse module families to product requirements or use cases
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v1.0.0 - Extracted traceability default mapping from traceability core]
// END_CHANGE_SUMMARY

// START_public_api

// START_TraceLinkSeed
#[derive(Debug, Clone, Copy)]
pub(super) struct TraceLinkSeed {
    pub(super) target_id: &'static str,
    pub(super) relationship: &'static str,
}
// END_TraceLinkSeed

// START_CONTRACT_module_trace_defaults
// PURPOSE: Map Synapse module families to product requirements or use cases for strict traceability adoption
// INPUTS: { module_id: &str }
// OUTPUTS: { Vec<TraceLinkSeed> }
// START_module_trace_defaults
pub(super) fn module_trace_defaults(module_id: &str) -> Vec<TraceLinkSeed> {
    let mut seeds = Vec::new();
    let mut add = |target_id, relationship| {
        seeds.push(TraceLinkSeed {
            target_id,
            relationship,
        })
    };
    if module_id.starts_with("M-INSTALL")
        || module_id.starts_with("M-BUILD")
        || module_id.starts_with("M-CI")
        || module_id == "M-TESTS-PARITY"
    {
        add("NFR-001", "traces_to");
    }
    if module_id.starts_with("M-CLI-SETUP")
        || module_id.starts_with("M-HOOK")
        || module_id == "M-HOOKS"
        || module_id == "M-PLUGIN"
        || module_id == "M-MAIN"
        || module_id == "M-LIB"
    {
        add("UC-001", "implements");
    }
    if module_id.starts_with("M-GRACE")
        || module_id.starts_with("M-MCP")
        || module_id.starts_with("M-AGENT")
        || module_id.starts_with("M-SKILLS")
        || module_id == "M-CAPABILITIES"
        || module_id == "M-CLI"
        || module_id.starts_with("M-CLI-GRACE")
    {
        add("UC-002", "implements");
        add("NFR-002", "traces_to");
    }
    if module_id.starts_with("M-TESTS-") && module_id != "M-TESTS-PARITY" {
        add("NFR-002", "traces_to");
    }
    if module_id.starts_with("M-INDEXER")
        || module_id.starts_with("M-GRAPHRAG")
        || module_id.starts_with("M-PROXY")
        || module_id.starts_with("M-CLI-CODE")
        || module_id.starts_with("M-CLI-RUNTIME")
        || matches!(
            module_id,
            "M-CONFIG" | "M-COMPRESS" | "M-DASHBOARD" | "M-TRACKING" | "M-UTILS"
        )
    {
        add("NFR-003", "traces_to");
    }
    seeds
}
// END_module_trace_defaults

// END_public_api
