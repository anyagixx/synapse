// MODULE_CONTRACT
// MODULE_ID: M-SKILLS-TYPES
// PURPOSE: Skill runtime types — metadata, schemas, requests, responses, and execution context
// SCOPE: SkillDef, SkillArg, SkillRequest, SkillResponse, SkillContext
// DEPENDS: M-CONFIG
// LINKS:
//   -> M-SKILLS (depends) - skill runtime facade

// START_MODULE_MAP
// SkillDef — Static definition of a callable skill tool
// SkillRequest — Invocation payload for skill execution
// SkillResponse — Structured text response for MCP output
// END_MODULE_MAP

// START_CHANGE_SUMMARY
// LAST_CHANGE: [v2.5.0 — Migrated semantic LINKS to typed format]
// END_CHANGE_SUMMARY

use crate::config::Config;
use std::path::PathBuf;

// START_public_api

// START_SkillArg
#[derive(Debug, Clone, Copy)]
pub struct SkillArg {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
}
// END_SkillArg

// START_SkillDef
#[derive(Debug, Clone, Copy)]
pub struct SkillDef {
    pub name: &'static str,
    pub description: &'static str,
    pub args: &'static [SkillArg],
}
// END_SkillDef

// START_SkillContext
#[derive(Debug, Clone)]
pub struct SkillContext {
    pub root: PathBuf,
    pub config: Config,
}
// END_SkillContext

// START_SkillRequest
#[derive(Debug, Clone)]
pub struct SkillRequest {
    pub name: String,
    pub arguments: serde_json::Value,
}
// END_SkillRequest

// START_SkillResponse
#[derive(Debug, Clone)]
pub struct SkillResponse {
    pub title: String,
    pub body: String,
}
// END_SkillResponse

impl SkillDef {
    // START_CONTRACT_SkillDef::input_schema
    // PURPOSE: Build MCP-compatible JSON schema for a skill definition
    // OUTPUTS: { serde_json::Value — object schema }
    // START_skill_def_input_schema
    pub fn input_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();
        for arg in self.args {
            properties.insert(
                arg.name.to_string(),
                serde_json::json!({
                    "type": "string",
                    "description": arg.description,
                }),
            );
            if arg.required {
                required.push(arg.name);
            }
        }

        serde_json::json!({
            "type": "object",
            "properties": properties,
            "required": required,
        })
    }
    // END_skill_def_input_schema
}

// END_public_api
