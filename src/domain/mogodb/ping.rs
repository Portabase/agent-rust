#![allow(dead_code)]

// use crate::domain::mogodb::connection::connect;
use crate::services::config::DatabaseConfig;
use anyhow::Result;
// use mongodb::bson::doc;
use std::collections::HashMap;

pub async fn run(cfg: DatabaseConfig, _env: HashMap<String, String>) -> Result<bool> {
    // let client = connect(cfg.clone()).await?;

    //
    // let db_name = if cfg.username.is_empty() { &cfg.database } else { "admin" };
    // let result = client.database(db_name).run_command(doc! { "ping": 1 }).await;
    //
    // match result {
    //     Ok(_) => Ok(true),
    //     Err(e) => Err(anyhow::anyhow!(
    //         "Failed to ping MongoDB server {}: {}",
    //         cfg.name,
    //         e
    //     )),
    // }
    Ok(true)
}
