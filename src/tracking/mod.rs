use crate::config::Config;
use std::path::PathBuf;

pub struct Tracker {
    config: Config,
}

impl Tracker {
    pub fn new(config: &Config) -> Self {
        Self { config: config.clone() }
    }

    pub fn db_path() -> anyhow::Result<PathBuf> {
        let data_dir = dirs::data_dir()
            .ok_or_else(|| anyhow::anyhow!("Cannot find data directory"))?
            .join("synapse");
        Ok(data_dir.join("tracking.db"))
    }

    pub async fn record(
        &self,
        cmd: &str,
        input_tokens: u32,
        output_tokens: u32,
    ) -> anyhow::Result<()> {
        if !self.config.tracking.enabled() {
            return Ok(());
        }
        let saved = input_tokens.saturating_sub(output_tokens);
        let savings_pct = if input_tokens > 0 {
            (saved as f64 / input_tokens as f64 * 100.0) as u32
        } else {
            0
        };
        let project = std::env::current_dir()
            .ok()
            .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
            .unwrap_or_default();

        let db_path = Self::db_path()?;
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS commands (
                    id INTEGER PRIMARY KEY,
                    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                    original_cmd TEXT NOT NULL,
                    input_tokens INTEGER NOT NULL,
                    output_tokens INTEGER NOT NULL,
                    saved_tokens INTEGER NOT NULL,
                    savings_pct REAL NOT NULL,
                    project_path TEXT DEFAULT ''
                );"
            ).ok();
            conn.execute(
                "INSERT INTO commands (original_cmd, input_tokens, output_tokens, saved_tokens, savings_pct, project_path)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![cmd, input_tokens, output_tokens, saved, savings_pct, project],
            ).ok();
        }
        Ok(())
    }

    pub async fn get_stats(&self) -> anyhow::Result<TrackingStats> {
        let db_path = Self::db_path()?;
        let mut stats = TrackingStats::default();

        if let Ok(conn) = rusqlite::Connection::open(&db_path) {
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS commands (
                    id INTEGER PRIMARY KEY,
                    timestamp TEXT NOT NULL DEFAULT (datetime('now')),
                    original_cmd TEXT NOT NULL,
                    input_tokens INTEGER NOT NULL,
                    output_tokens INTEGER NOT NULL,
                    saved_tokens INTEGER NOT NULL,
                    savings_pct REAL NOT NULL,
                    project_path TEXT DEFAULT ''
                );"
            ).ok();

            if let Ok(mut stmt) = conn.prepare(
                "SELECT COUNT(*), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0),
                        COALESCE(SUM(saved_tokens),0), COALESCE(AVG(savings_pct),0)
                 FROM commands"
            ) {
                if let Ok(row) = stmt.query_row([], |row| {
                    stats.total_commands = row.get(0)?;
                    stats.total_input_tokens = row.get(1)?;
                    stats.total_output_tokens = row.get(2)?;
                    stats.total_saved_tokens = row.get(3)?;
                    stats.avg_savings_pct = row.get(4)?;
                    Ok(())
                }) {
                    let _ = row;
                }
            }
        }
        Ok(stats)
    }
}

#[derive(serde::Serialize, Default, Debug)]
pub struct TrackingStats {
    pub total_commands: u64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_saved_tokens: u64,
    pub avg_savings_pct: f64,
}
