use crate::services::config::DatabaseConfig;
use anyhow::Context;
use std::collections::HashMap;
use std::process::Command;

pub async fn run(cfg: DatabaseConfig, env: HashMap<String, String>) -> anyhow::Result<bool> {
    let output = Command::new("mysqladmin")
        .arg("--host")
        .arg(cfg.host)
        .arg("--port")
        .arg(cfg.port.to_string())
        .arg("--user")
        .arg(cfg.username)
        .arg("ping")
        .envs(env)
        .output()
        .with_context(|| format!("Failed to ping MySQL server {}", cfg.name))?;
    Ok(output.status.success())
}
