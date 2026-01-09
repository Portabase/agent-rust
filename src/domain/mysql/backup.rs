use anyhow::{Context, Result};
use tracing::debug;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::services::config::DatabaseConfig;

pub async fn run(
    cfg: DatabaseConfig,
    backup_dir: PathBuf,
    env: HashMap<String, String>,
    file_extension: &'static str,
) -> Result<PathBuf> {
    tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        debug!("Starting backup for database {}", cfg.name);

        let file_path = backup_dir.join(format!("{}{}", cfg.generated_id, file_extension));

        let output = Command::new("mysqldump")
            .arg("--host")
            .arg(cfg.host)
            .arg("--port")
            .arg(cfg.port.to_string())
            .arg("--user")
            .arg(cfg.username)
            .arg("--routines")
            .arg("--events")
            .arg("--triggers")
            .arg("--verbose")
            .arg("--single-transaction")
            .arg("--quick")
            .arg("--add-drop-database")
            .arg("--databases")
            .arg(cfg.database)
            .arg("-r")
            .arg(&file_path)
            .envs(env)
            .output()
            .with_context(|| format!("Failed to run mysqldump for {}", cfg.name))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("MySQL backup failed for {}: {}", cfg.name, stderr);
        }

        Ok(file_path)
    })
    .await?
}
