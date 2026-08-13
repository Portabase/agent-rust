use crate::services::config::{build_config, DatabaseConfig, InputDatabaseConfig};
use crate::services::dashboard_config::merge;
use crate::services::dashboard_config::{load_cache, persist_cache};

fn cfg(name: &str, gen_id: &str, host: &str) -> DatabaseConfig {
    let json = format!(
        r#"{{ "name": "{name}", "type": "postgresql", "database": "app",
               "username": "u", "password": "p", "port": 5432,
               "host": "{host}", "generated_id": "{gen_id}" }}"#
    );
    let input: InputDatabaseConfig = serde_json::from_str(&json).unwrap();
    build_config(input).unwrap()
}

const ID_A: &str = "16678159-ff7e-4c97-8c83-0adeff214681";
const ID_B: &str = "16678124-ff7e-4c97-8c83-0adeff214681";

#[test]
fn merge_keeps_local_only_databases() {
    let local = vec![cfg("local-a", ID_A, "local-host")];
    let merged = merge(&local, &[]);
    assert_eq!(merged.databases.len(), 1);
    assert_eq!(merged.databases[0].host, "local-host");
}

#[test]
fn merge_appends_dashboard_only_databases() {
    let local = vec![cfg("local-a", ID_A, "local-host")];
    let dashboard = vec![cfg("dash-b", ID_B, "dash-host")];
    let merged = merge(&local, &dashboard);
    assert_eq!(merged.databases.len(), 2);
    assert!(merged.databases.iter().any(|d| d.generated_id == ID_B));
}

#[test]
fn merge_dashboard_wins_on_id_collision() {
    let local = vec![cfg("local-a", ID_A, "local-host")];
    let dashboard = vec![cfg("dash-a", ID_A, "dash-host")];
    let merged = merge(&local, &dashboard);
    assert_eq!(merged.databases.len(), 1);
    assert_eq!(merged.databases[0].host, "dash-host"); // dashboard wins
    assert_eq!(merged.databases[0].name, "dash-a");
}

#[test]
fn cache_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dashboard_databases.json");

    let dbs = vec![cfg("dash-a", ID_A, "dash-host")];
    persist_cache(&path, &dbs).unwrap();

    let loaded = load_cache(&path);
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].generated_id, ID_A);
    assert_eq!(loaded[0].host, "dash-host");
}

#[test]
fn load_cache_missing_file_is_empty() {
    let loaded = load_cache(std::path::Path::new("/nonexistent/dashboard_databases.json"));
    assert!(loaded.is_empty());
}

#[test]
fn load_cache_corrupt_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dashboard_databases.json");
    std::fs::write(&path, b"{ this is not valid json").unwrap();

    let loaded = load_cache(&path);
    assert!(loaded.is_empty());
}

#[test]
fn persist_cache_leaves_no_tmp_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dashboard_databases.json");
    persist_cache(&path, &[cfg("dash-a", ID_A, "h")]).unwrap();

    let tmp = path.with_extension("json.tmp");
    assert!(!tmp.exists(), "temp file should have been renamed away");
    assert!(path.exists());
}
