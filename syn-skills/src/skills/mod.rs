// MODULE_CONTRACT
// MODULE_ID: M-SKILLS
// PURPOSE: Skill runtime facade — exposes 16 first-class GRACE skill tools for MCP and CLI integration
// SCOPE: SkillEngine facade, registry exports, skill metadata and execution surface
// DEPENDS: M-CONFIG, M-GRACE-LAYOUT, M-SKILLS-ENGINE, M-SKILLS-REGISTRY, M-SKILLS-TYPES
// LINKS:
//   -> M-MCP-SERVER (depends) - MCP exposure for skill tools

// START_MODULE_MAP
// SkillEngine — Skill execution engine
// registry — Skill metadata registry
// types — Shared skill runtime types
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.8.0 — Updated skill facade count for run history]
// END_CHANGE_SUMMARY

// START_CONTRACT_public_api
// PURPOSE: Export skill engine, registry, types, and public runtime aliases
// OUTPUTS: { SkillEngine }, { SkillDef }, { SkillRequest }, { SkillResponse }
mod engine;
pub mod registry;
pub mod types;

pub use engine::SkillEngine;
pub use types::{SkillDef, SkillRequest, SkillResponse};
