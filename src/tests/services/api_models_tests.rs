use serde_json::json;

use crate::services::api::models::agent::backup::{BackupResponse, BackupUploadResponse};
use crate::services::api::models::agent::restore::ResultRestoreResponse;
use crate::services::api::models::agent::status::{DatabaseStatus, PingResult};

#[test]
fn backup_response_deserializes_nested_backup_id() {
    let response: BackupResponse = serde_json::from_value(json!({
        "message": "created",
        "backup": {
            "id": "backup-123"
        }
    }))
    .unwrap();

    assert_eq!(response.message, "created");
    assert_eq!(response.backup.id, "backup-123");
}

#[test]
fn backup_upload_response_deserializes_storage_payload() {
    let response: BackupUploadResponse = serde_json::from_value(json!({
        "message": "uploaded",
        "backupStorage": {
            "id": "storage-456"
        }
    }))
    .unwrap();

    assert_eq!(response.message, "uploaded");
    assert_eq!(response.backup_storage.id, "storage-456");
}

#[test]
fn restore_response_deserializes_status() {
    let response: ResultRestoreResponse = serde_json::from_value(json!({
        "message": "ok",
        "status": true
    }))
    .unwrap();

    assert_eq!(response.message, "ok");
    assert!(response.status);
}

#[test]
fn ping_result_deserializes_and_normalizes_storage_config_keys() {
    let payload = json!({
        "agent": {
            "id": "agent-1",
            "lastContact": "2026-03-22T10:00:00Z"
        },
        "databases": [{
            "dbms": "postgres",
            "generatedId": "db-1",
            "storages": [{
                "id": "storage-1",
                "provider": "s3",
                "config": {
                    "bucketName": "agent-backups",
                    "nestedConfig": {
                        "regionName": "eu-west-3"
                    },
                    "allowedRegions": [
                        { "regionCode": "eu-west-3" }
                    ]
                }
            }],
            "encrypt": true,
            "data": {
                "backup": {
                    "action": true,
                    "cron": "*/5 * * * *"
                },
                "restore": {
                    "action": false,
                    "file": null,
                    "metaFile": null
                }
            }
        }]
    });

    let result: PingResult = serde_json::from_value(payload).unwrap();
    let storage = &result.databases[0].storages[0];

    assert_eq!(result.agent.id, "agent-1");
    assert_eq!(result.agent.last_contact, "2026-03-22T10:00:00Z");
    assert_eq!(result.databases[0].generated_id, "db-1");
    assert_eq!(storage.provider, "s3");
    assert_eq!(
        storage.config["bucket_name"].as_str(),
        Some("agent-backups")
    );
    assert_eq!(
        storage.config["nested_config"]["region_name"].as_str(),
        Some("eu-west-3")
    );
    assert_eq!(
        storage.config["allowed_regions"][0]["region_code"].as_str(),
        Some("eu-west-3")
    );
    assert_eq!(
        result.databases[0].data.backup.cron.as_deref(),
        Some("*/5 * * * *")
    );
    assert!(result.databases[0].data.backup.action);
    assert!(!result.databases[0].data.restore.action);
    assert!(result.databases[0].data.restore.file.is_none());
    assert!(result.databases[0].data.restore.meta_file.is_none());
}

#[test]
fn database_status_legacy_plaintext_storages() {
    let status: DatabaseStatus = serde_json::from_value(json!({
        "dbms": "postgres",
        "generatedId": "gen-1",
        "storages": [ { "id": "s1", "config": { "bucket": "b" }, "provider": "s3" } ],
        "encrypt": true,
        "data": {
            "backup": { "action": false, "cron": null },
            "restore": { "action": false, "file": null, "metaFile": null, "size": null }
        }
    })).unwrap();

    assert_eq!(status.storages.len(), 1);
    assert_eq!(status.storages_encrypted, None);
    assert!(status.storages_ciphertext.is_none());
}

#[test]
fn database_status_encrypted_envelope() {
    let status: DatabaseStatus = serde_json::from_value(json!({
        "dbms": "postgres",
        "generatedId": "gen-1",
        "storages": [],
        "storages_encrypted": true,
        "storages_ciphertext": "AQIDBA==",
        "encrypt": true,
        "data": {
            "backup": { "action": true, "cron": null },
            "restore": { "action": false, "file": null, "metaFile": null, "size": null }
        }
    })).unwrap();

    assert!(status.storages.is_empty());
    assert_eq!(status.storages_encrypted, Some(true));
    assert_eq!(status.storages_ciphertext.as_deref(), Some("AQIDBA=="));
}

#[test]
fn database_status_defaults_config_fields_absent() {
    let json = r#"{
        "dbms": "postgresql",
        "generatedId": "16678159-ff7e-4c97-8c83-0adeff214681",
        "encrypt": false,
        "data": { "backup": { "action": false, "cron": null },
                  "restore": { "action": false, "file": null, "metaFile": null, "size": null } }
    }"#;
    let status: crate::services::api::models::agent::status::DatabaseStatus =
        serde_json::from_str(json).unwrap();
    assert_eq!(status.config_encrypted, None);
    assert!(status.config_ciphertext.is_none());
    assert!(status.resolved_config.is_none());
}

#[test]
fn resolve_dashboard_config_decrypts_full_entry() {
    use crate::services::status::resolve_dashboard_config;
    use base64::{engine::general_purpose, Engine};

    // 32-byte master key, base64 STANDARD (matches decrypt_json_gcm).
    let master_key_b64 = general_purpose::STANDARD.encode([7u8; 32]);

    // Full agent-entry shape the dashboard encrypts.
    let entry = r#"{
        "name": "Dashboard PG",
        "type": "postgresql",
        "database": "app",
        "username": "postgres",
        "password": "s3cret",
        "port": 5432,
        "host": "10.0.0.10",
        "generated_id": "16678159-ff7e-4c97-8c83-0adeff214681"
    }"#;
    let ciphertext = encrypt_json_gcm(entry.as_bytes(), &master_key_b64);

    let mut status: crate::services::api::models::agent::status::DatabaseStatus =
        serde_json::from_str(
            r#"{
                "dbms": "postgresql",
                "generatedId": "16678159-ff7e-4c97-8c83-0adeff214681",
                "encrypt": false,
                "config_encrypted": true,
                "config_ciphertext": "PLACEHOLDER",
                "data": { "backup": { "action": false, "cron": null },
                          "restore": { "action": false, "file": null, "metaFile": null, "size": null } }
            }"#,
        )
        .unwrap();
    status.config_ciphertext = Some(ciphertext);

    resolve_dashboard_config(&mut status, &master_key_b64).unwrap();

    let cfg = status.resolved_config.expect("resolved");
    assert_eq!(cfg.name, "Dashboard PG");
    assert_eq!(cfg.password, "s3cret");
    assert_eq!(cfg.host, "10.0.0.10");
    assert_eq!(cfg.db_type.as_str(), "postgresql");
}

#[test]
fn resolve_dashboard_config_noop_when_not_encrypted() {
    use crate::services::status::resolve_dashboard_config;
    let mut status: crate::services::api::models::agent::status::DatabaseStatus =
        serde_json::from_str(
            r#"{
                "dbms": "postgresql",
                "generatedId": "16678159-ff7e-4c97-8c83-0adeff214681",
                "encrypt": false,
                "data": { "backup": { "action": false, "cron": null },
                          "restore": { "action": false, "file": null, "metaFile": null, "size": null } }
            }"#,
        )
        .unwrap();
    resolve_dashboard_config(&mut status, "unused").unwrap();
    assert!(status.resolved_config.is_none());
}

fn encrypt_json_gcm(plaintext: &[u8], master_key_b64: &str) -> String {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use base64::{engine::general_purpose, Engine};

    let key_bytes = general_purpose::STANDARD.decode(master_key_b64).unwrap();
    let key = Key::<Aes256Gcm>::try_from(key_bytes.as_slice()).unwrap();
    let cipher = Aes256Gcm::new(&key);
    let nonce_bytes = [0u8; 12];
    let nonce = Nonce::try_from(&nonce_bytes[..]).unwrap();
    let ct = cipher.encrypt(&nonce, plaintext).unwrap();
    let mut data = nonce_bytes.to_vec();
    data.extend_from_slice(&ct);
    general_purpose::STANDARD.encode(data)
}
