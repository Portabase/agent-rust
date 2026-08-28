use super::logger::JobLogger;
use super::models::BackupResult;
use super::service::BackupService;

use crate::domain::factory::DatabaseFactory;
use crate::services::config::DatabaseConfig;
use crate::utils::retry::{RetryPolicy, retry};

use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tracing::error;

impl BackupService {
    pub async fn run(cfg: DatabaseConfig, tmp_path: &Path, logger: Arc<JobLogger>) -> Result<BackupResult> {
        let db = DatabaseFactory::create_for_backup(cfg.clone()).await;

        let generated_id = cfg.generated_id.clone();
        let db_type = cfg.db_type.clone();

        let reachable = match db.ping().await {
            Ok(v) => v,
            Err(e) => {
                error!("Ping failed: {}", e);
                logger.log("error", format!("Ping failed: {}", e));
                return Err(e.into());
            }
        };

        logger.log("info", format!("Database reachable: {}", reachable));

        if !reachable {
            logger.log("error", "Database unreachable, backup aborted");
            return Ok(BackupResult {
                generated_id,
                db_type,
                status: "failed".into(),
                backup_file: None,
                code: None,
            });
        }

        let policy = RetryPolicy::default();

        let db_ref = &db;
        let logger_ref = &logger;

        let outcome = retry("Database backup", &logger, &policy, move |attempt| {
            let dir = tmp_path.join(format!("attempt-{attempt}"));

            async move {
                if let Err(e) = tokio::fs::create_dir_all(&dir).await {
                    return Err(anyhow::Error::from(e));
                }

                match db_ref.backup(&dir, Arc::clone(logger_ref)).await {
                    Ok(f) => Ok(f),
                    Err(e) => {
                        let _ = tokio::fs::remove_dir_all(&dir).await;
                        Err(e)
                    }
                }
            }
        })
        .await;

        match outcome {
            Ok(file) => Ok(BackupResult {
                generated_id,
                db_type,
                status: "success".into(),
                backup_file: Some(file),
                code: None,
            }),

            Err(e) if e.to_string() == "backup_already_in_progress" => {
                logger.log("warn", "Backup already in progress");
                Ok(BackupResult {
                    generated_id,
                    db_type,
                    status: "failed".into(),
                    backup_file: None,
                    code: Some("backup_already_in_progress".into()),
                })
            }

            Err(e) => {
                logger.log("error", format!("Backup failed: {}", e));
                Ok(BackupResult {
                    generated_id,
                    db_type,
                    status: "failed".into(),
                    backup_file: None,
                    code: None,
                })
            }
        }
    }
}
