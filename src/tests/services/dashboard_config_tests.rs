use crate::services::config::{build_config, DatabaseConfig, InputDatabaseConfig};
use crate::services::dashboard_config::merge;

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
