use crate::services::config::DatabaseConfig;
use anyhow::Result;
use std::process::Command;

pub fn connection_args(cfg: &DatabaseConfig) -> Vec<String> {
    let protocol = cfg
        .options
        .get("protocol")
        .and_then(|v| v.as_str())
        .unwrap_or("tcp");

    let mut args = vec![
        format!("--protocol={}", protocol),
        "--host".to_string(),
        cfg.host.clone(),
        "--port".to_string(),
        cfg.port.to_string(),
        "--user".to_string(),
        cfg.username.clone(),
    ];

    if let Some(socket) = cfg.options.get("socket").and_then(|v| v.as_str()) {
        args.push(format!("--socket={}", socket));
    }

    args
}

pub async fn server_version(cfg: &DatabaseConfig) -> Result<String> {
    let output = Command::new("mysql")
        .args(connection_args(cfg))
        .arg("-e")
        .arg("SELECT VERSION();")
        .env("MYSQL_PWD", &cfg.password)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Version query failed: {}", stderr);
    }

    let version = String::from_utf8_lossy(&output.stdout)
        .lines()
        .nth(1)
        .unwrap_or_default()
        .trim()
        .to_string();

    Ok(version)
}
