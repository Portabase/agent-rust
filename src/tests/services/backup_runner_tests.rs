use crate::services::backup::BackupService;
use crate::services::backup::logger::JobLogger;
use crate::services::config::{DatabaseConfig, DbType};
use crate::tests::init_tracing_for_test;

use std::collections::HashMap;
use std::sync::Arc;
use tempfile::TempDir;

fn sqlite_config(path: &str) -> DatabaseConfig {
    DatabaseConfig {
        name: "retry-test".to_string(),
        database: String::new(),
        db_type: DbType::Sqlite,
        username: String::new(),
        password: String::new(),
        port: 0,
        host: String::new(),
        generated_id: "retry-test-gen".to_string(),
        path: path.to_string(),
        max_packet_size: String::new(),
        volume_name: String::new(),
        container_name: None,
        options: HashMap::new(),
    }
}

#[tokio::test]
async fn a_failing_backup_is_retried_and_leaves_no_attempt_directory() {
    init_tracing_for_test();

    let temp_dir = TempDir::new().unwrap();
    let tmp_path = temp_dir.path();
    let logger = Arc::new(JobLogger::new());

    let cfg = sqlite_config("/nonexistent/definitely-not-here.sqlite");

    let result = BackupService::run(cfg, tmp_path, Arc::clone(&logger))
        .await
        .unwrap();

    assert_eq!(result.status, "failed");
    assert!(result.backup_file.is_none());

    let entries = Arc::try_unwrap(logger).unwrap().into_entries();
    assert_eq!(
        entries.iter().filter(|e| e.level == "warn").count(),
        2,
        "expected one warn per non-final failed attempt"
    );
    assert!(
        entries
            .iter()
            .any(|e| e.level == "error" && e.message.starts_with("Database backup failed after 3 attempts")),
        "expected a single terminal error naming the attempt count"
    );

    let leftovers: Vec<_> = std::fs::read_dir(tmp_path)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("attempt-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "failed attempt directories must be cleaned up, found {:?}",
        leftovers.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
}
