use crate::config::Config;

pub struct SkillEngine;

impl SkillEngine {
    pub fn new(_config: &Config) -> Self {
        SkillEngine
    }

    pub async fn execute(&self, _skill: &str, _args: &[String]) -> anyhow::Result<String> {
        anyhow::bail!("Skill engine not yet implemented")
    }
}
