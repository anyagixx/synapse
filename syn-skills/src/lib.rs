// MODULE_CONTRACT
// MODULE_ID: M-WORKSPACE-SKILLS
// PURPOSE: Skills crate root — GRACE workflow skill engine (16 skills), capabilities registry, and skill type definitions
// DEPENDS: M-WORKSPACE-CORE, M-WORKSPACE-ENGINE, M-WORKSPACE-RUN
// LINKS:
//   → Phase-97 (implements) — workspace split
//   ← V-M-WORKSPACE-SKILLS (verified_by) — verification shard

#![allow(clippy::if_same_then_else, clippy::new_without_default)]

// START_MODULE_MAP
// skills — Skill engine, registry (16 GRACE workflow skills), and type definitions
// capabilities — Machine-readable capability registry (commands, MCP tools, verify checks, platforms)
// SkillEngine — Re-exported skill execution engine
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.7.0 — Extracted from synapse-agent monolith into syn-skills workspace crate]
// END_CHANGE_SUMMARY

pub mod skills;
pub mod capabilities;

// START_CONTRACT_public_api
// PURPOSE: Re-export skill engine for downstream consumers
// OUTPUTS: { SkillEngine — GRACE workflow skill execution engine }
// LINKS:
//   → NFR-002 (traces_to) — type exports enable reliable downstream usage
// START_public_api
pub use skills::SkillEngine;
// END_public_api
