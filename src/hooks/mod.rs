use crate::config::Config;

pub struct HookManager;

impl HookManager {
    pub fn new(_config: &Config) -> Self {
        HookManager
    }

    pub async fn install(&self, _agent: &str) -> anyhow::Result<()> {
        anyhow::bail!("Hook installation not yet implemented")
    }

    pub async fn uninstall(&self, _agent: &str) -> anyhow::Result<()> {
        anyhow::bail!("Hook removal not yet implemented")
    }
}
