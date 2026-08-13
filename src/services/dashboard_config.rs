#![allow(dead_code)]

use crate::services::api::models::agent::status::PingResult;
use crate::services::config::{DatabaseConfig, DatabasesConfig};
use std::path::Path;

/// Merge local (file) databases with dashboard-provided ones. Dashboard wins on
/// a matching `generated_id`; otherwise dashboard entries are appended. Local
/// order is preserved.
pub fn merge(local: &[DatabaseConfig], dashboard: &[DatabaseConfig]) -> DatabasesConfig {
    let mut databases: Vec<DatabaseConfig> = local.to_vec();
    for d in dashboard {
        if let Some(slot) = databases
            .iter_mut()
            .find(|c| c.generated_id == d.generated_id)
        {
            *slot = d.clone();
        } else {
            databases.push(d.clone());
        }
    }
    DatabasesConfig { databases }
}

/// The authoritative dashboard set for this cycle: every returned database whose
/// `config_ciphertext` decrypted successfully into a `resolved_config`.
pub fn collect_configs(ping: &PingResult) -> Vec<DatabaseConfig> {
    ping.databases
        .iter()
        .filter_map(|db| db.resolved_config.clone())
        .collect()
}

/// Load the persisted dashboard set. A missing or corrupt file is treated as an
/// empty set (logged), so a bad cache can never crash startup — it is rebuilt
/// from the next `/status` response.
pub fn load_cache(path: &Path) -> Vec<DatabaseConfig> {
    let contents = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    match serde_json::from_str::<DatabasesConfig>(&contents) {
        Ok(cfg) => cfg.databases,
        Err(e) => {
            tracing::warn!("Dashboard cache at {:?} is corrupt ({e}); ignoring", path);
            Vec::new()
        }
    }
}

/// Persist the dashboard set atomically: write a sibling `*.tmp` then rename over
/// the target, so a crash mid-write can never leave a half-written cache.
pub fn persist_cache(path: &Path, databases: &[DatabaseConfig]) -> std::io::Result<()> {
    let wrapper = DatabasesConfig {
        databases: databases.to_vec(),
    };
    let json = serde_json::to_string_pretty(&wrapper)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, json)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}
