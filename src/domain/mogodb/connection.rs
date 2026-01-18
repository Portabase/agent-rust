use crate::services::config::DatabaseConfig;
use anyhow::{Context, Result};
use mongodb::{Client};

pub async fn connect(cfg: DatabaseConfig) -> Result<Client> {
    let uri = if cfg.username.is_empty() && cfg.password.is_empty() {
        // MongoDB sans auth
        format!("mongodb://{}:{}", cfg.host, cfg.port)
    } else {
        // MongoDB avec auth
        format!(
            "mongodb://{}:{}@{}:{}/?authSource=admin",
            cfg.username, cfg.password, cfg.host, cfg.port
        )
    };

    let mut options = mongodb::options::ClientOptions::parse(&uri)
        .await
        .with_context(|| format!("Failed to parse MongoDB URI for {}", cfg.name))?;

    options.app_name = Some("my-rust-app".to_string());

    let client = Client::with_options(options)
        .with_context(|| format!("Failed to create MongoDB client for {}", cfg.name))?;

    Ok(client)
}


pub fn select_mongo_path() -> std::path::PathBuf {
    "/usr/local/mongodb/bin".to_string().into()
}