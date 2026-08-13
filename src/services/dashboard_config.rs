#![allow(dead_code)]

use crate::services::api::models::agent::status::PingResult;
use crate::services::config::{DatabaseConfig, DatabasesConfig};

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
