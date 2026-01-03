#![allow(dead_code)]

use crate::domain::postgres::PostgresDatabase;
use crate::services::config::DatabaseConfig;
use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::domain::mysql::MySQLDatabase;

#[async_trait::async_trait]
pub trait Database: Send + Sync {
    fn file_extension(&self) -> &'static str;

    async fn ping(&self) -> Result<bool>;
    async fn backup(&self, backup_dir: &Path) -> Result<PathBuf>;
    async fn restore(&self, restore_file: &Path) -> Result<()>;
}

pub struct DatabaseFactory;

impl DatabaseFactory {
    pub async fn create_for_backup(cfg: DatabaseConfig) -> Arc<dyn Database> {
        match cfg.db_type.as_str() {
            "postgresql" => {
                let format = PostgresDatabase::detect_format_from_size(&cfg).await;
                Arc::new(PostgresDatabase::new(cfg, format))
            }
            "mysql" => Arc::new(MySQLDatabase::new(cfg)),
            _ => panic!("Unsupported DB type: {}", cfg.db_type),
        }
    }

    pub async fn create_for_restore(cfg: DatabaseConfig, restore_file: &Path) -> Arc<dyn Database> {
        match cfg.db_type.as_str() {
            "postgresql" => {
                let format = PostgresDatabase::detect_format_from_file(restore_file);
                Arc::new(PostgresDatabase::new(cfg, format))
            }
            "mysql" => Arc::new(MySQLDatabase::new(cfg)),
            _ => panic!("Unsupported DB type: {}", cfg.db_type),
        }
    }
}
