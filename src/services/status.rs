#![allow(dead_code)]

use crate::core::context::Context;
use crate::domain::factory::DatabaseFactory;
use crate::services::api::endpoints::status::DatabasePayload;
use crate::services::api::models::agent::status::DatabaseStatus;
use crate::services::api::models::agent::status::DatabaseStorage;
use crate::services::api::models::agent::status::PingResult;
use crate::services::config::{build_config, DatabaseConfig, InputDatabaseConfig};
use crate::settings::CONFIG;
use crate::utils::file::decrypt_json_gcm;
use futures_util::future::try_join_all;
use reqwest::Client;
use std::error::Error;
use std::sync::Arc;
use tracing::info;

/// Decrypt a database's dashboard `config_ciphertext` (if present) into
/// `resolved_config`. AES-256-GCM envelope, same as `storages`. Errors are
/// returned so the caller can log-and-skip a single database.
pub fn resolve_dashboard_config(
    status: &mut DatabaseStatus,
    master_key_b64: &str,
) -> Result<(), String> {
    if status.config_encrypted != Some(true) {
        return Ok(());
    }
    let ciphertext = status
        .config_ciphertext
        .as_deref()
        .ok_or("config_encrypted set but config_ciphertext missing")?;

    let plaintext = decrypt_json_gcm(ciphertext, master_key_b64)
        .map_err(|e| format!("Failed to decrypt config: {e}"))?;
    let input: InputDatabaseConfig = serde_json::from_slice(&plaintext)
        .map_err(|e| format!("Failed to parse decrypted config: {e}"))?;
    status.resolved_config = Some(build_config(input)?);
    Ok(())
}

pub struct StatusService {
    ctx: Arc<Context>,
    client: Client,
}

impl StatusService {
    pub fn new(ctx: Arc<Context>) -> Self {
        StatusService {
            ctx,
            client: Client::new(),
        }
    }

    pub async fn ping(&self, databases: &[DatabaseConfig]) -> Result<PingResult, Box<dyn Error>> {
        let edge_key = &self.ctx.edge_key;

        let databases_payload: Vec<DatabasePayload> =
            try_join_all(databases.into_iter().map(|db| async move {
                let db_engine = DatabaseFactory::create_for_backup(db.clone()).await;

                let reachable = db_engine.ping().await?;
                info!("Ping {} => {:?}", db.name, reachable);

                Ok::<DatabasePayload, anyhow::Error>(DatabasePayload {
                    name: &db.name,
                    dbms: &db.db_type.as_str(),
                    generated_id: &db.generated_id,
                    ping_status: reachable,
                })
            }))
            .await?;

        let version_str = CONFIG.app_version.as_str();
        let mut result = self
            .ctx
            .api
            .agent_status(&edge_key.agent_id, &version_str, databases_payload)
            .await?
            .unwrap();

        for db in result.databases.iter_mut() {
            if db.storages_encrypted == Some(true) {
                let ciphertext = db
                    .storages_ciphertext
                    .as_deref()
                    .ok_or("storages_encrypted set but storages_ciphertext missing")?;

                let plaintext = decrypt_json_gcm(ciphertext, &edge_key.master_key_b64)
                    .map_err(|e| format!("Failed to decrypt storages: {e}"))?;

                db.storages = serde_json::from_slice::<Vec<DatabaseStorage>>(&plaintext)
                    .map_err(|e| format!("Failed to parse decrypted storages: {e}"))?;
            }

            if let Err(e) = resolve_dashboard_config(db, &edge_key.master_key_b64) {
                tracing::warn!("Skipping dashboard config for {}: {e}", db.generated_id);
            }
        }
        Ok(result)
    }
}
