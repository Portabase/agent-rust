use crate::domain::factory::DatabaseFactory;
use crate::domain::mysql::connection::connection_args;
use crate::services::config::{DatabaseConfig, DbType};
use crate::tests::init_tracing_for_test;
use crate::utils::compress::{compress_to_tar_gz_large, decompress_large_tar_gz};
use oauth2::url;
use std::path::PathBuf;
use tempfile::TempDir;
use testcontainers::runners::AsyncRunner;
use testcontainers::{ContainerAsync, ImageExt};
use testcontainers_modules::mysql::Mysql;
use tracing::{error, info};
use url::Host;

async fn create_config() -> (ContainerAsync<Mysql>, DatabaseConfig) {
    let container = Mysql::default().with_tag("8.1").start().await.unwrap();

    let host = container
        .get_host()
        .await
        .unwrap_or(Host::parse("127.0.0.1").unwrap());

    let port = container.get_host_port_ipv4(3306).await.unwrap_or(3306);

    let config = DatabaseConfig {
        name: "Test MySQL".to_string(),
        database: "test".to_string(),
        db_type: DbType::Mysql,
        username: "root".to_string(),
        password: "".to_string(),
        port,
        host: host.to_string(),
        generated_id: "0f1bb8f2-35a0-4c91-8098-e36873d3ce31".to_string(),
        path: "".to_string(),
        max_packet_size: "512M".to_string(),
        volume_name: "".to_string(),
        container_name: None,
        options: std::collections::HashMap::new(),
    };

    (container, config)
}

#[tokio::test]
async fn mysql_ping_test() {
    init_tracing_for_test();

    let (_container, config) = create_config().await;

    let db = DatabaseFactory::create_for_backup(config.clone()).await;
    let reachable = db.ping().await.unwrap_or(false);

    assert!(reachable);
}

#[tokio::test]
async fn mysql_backup_restore_test() {
    init_tracing_for_test();

    let (_container, config) = create_config().await;

    let temp_dir = TempDir::new().unwrap();
    let backup_path = temp_dir.path();

    let db = DatabaseFactory::create_for_backup(config.clone()).await;
    let file_path = db.backup(backup_path, std::sync::Arc::new(crate::services::backup::logger::JobLogger::new())).await.unwrap();

    assert!(file_path.is_file());

    let compression = compress_to_tar_gz_large(&file_path, std::sync::Arc::new(crate::services::backup::logger::JobLogger::new())).await.unwrap();
    assert!(compression.compressed_path.is_file());

    let files = decompress_large_tar_gz(compression.compressed_path.as_path(), temp_dir.path())
        .await
        .unwrap();

    let backup_file: PathBuf = if files.len() == 1 {
        files[0].clone()
    } else {
        "".into()
    };

    let db = DatabaseFactory::create_for_restore(config.clone(), &backup_file).await;
    let reachable = db.ping().await.unwrap_or(false);

    info!("Reachable: {}", reachable);
    assert!(reachable);

    match db.restore(&backup_file, std::sync::Arc::new(crate::services::backup::logger::JobLogger::new())).await {
        Ok(_) => {
            info!("Restore succeeded for {}", config.generated_id);
            assert!(true)
        }
        Err(e) => {
            error!("Restore failed for {}: {:?}", config.generated_id, e);
            assert!(false)
        }
    }
}

fn tunnelled_config(options: serde_json::Value) -> DatabaseConfig {
    DatabaseConfig {
        name: "my-db".to_string(),
        database: "my-db".to_string(),
        db_type: DbType::Mysql,
        username: "my-db-user".to_string(),
        password: "my-db-password".to_string(),
        port: 3306,
        host: "localhost".to_string(),
        generated_id: "16678159-ff7e-4c97-8c83-0adeff214681".to_string(),
        path: "".to_string(),
        max_packet_size: "512M".to_string(),
        volume_name: "".to_string(),
        container_name: None,
        options: serde_json::from_value(options).unwrap(),
    }
}

#[test]
fn connection_args_force_tcp_for_localhost() {
    // A `localhost` host makes the clients pick a Unix socket and ignore `--port`,
    // which breaks databases reached through an SSH tunnel.
    let cfg = tunnelled_config(serde_json::json!({}));

    assert_eq!(
        connection_args(&cfg),
        vec![
            "--protocol=tcp",
            "--host",
            "localhost",
            "--port",
            "3306",
            "--user",
            "my-db-user",
        ]
    );
}

#[test]
fn connection_args_allow_socket_opt_in() {
    let cfg = tunnelled_config(serde_json::json!({
        "protocol": "socket",
        "socket": "/var/run/mysqld/mysqld.sock",
    }));

    let args = connection_args(&cfg);

    assert_eq!(args[0], "--protocol=socket");
    assert_eq!(args.last().unwrap(), "--socket=/var/run/mysqld/mysqld.sock");
}
