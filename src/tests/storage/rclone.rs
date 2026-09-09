use crate::services::api::models::agent::status::DatabaseStorage;
use crate::services::storage::providers::rclone::helpers::{remote_target, validate_config};
use crate::services::storage::providers::rclone::models::RcloneProviderConfig;
use crate::tests::init_tracing_for_test;
use crate::utils::file::full_file_path;

const OVH_CONFIG: &str = "[ovhcloud-rbx]\n\
                          type = s3\n\
                          provider = OVHcloud\n\
                          access_key_id = my_access\n\
                          secret_access_key = my_secret\n\
                          region = rbx\n\
                          endpoint = s3.rbx.io.cloud.ovh.net\n\
                          acl = private\n";

#[test]
fn config_deserializes_from_dashboard_camel_case() {
    init_tracing_for_test();

    // Exactly the shape the dashboard puts on the wire: camelCase keys inside
    // `config`, converted to snake_case by `deserialize_snake_case`.
    let storage: DatabaseStorage = serde_json::from_value(serde_json::json!({
        "id": "storage-1",
        "provider": "rclone",
        "folderName": "backups",
        "config": {
            "configText": OVH_CONFIG,
            "remoteName": "ovhcloud-rbx",
            "remotePath": "my-bucket",
        }
    }))
    .unwrap();

    let config: RcloneProviderConfig = storage.config.try_into().unwrap();

    assert_eq!(config.remote_name, "ovhcloud-rbx");
    assert_eq!(config.remote_path, "my-bucket");
    assert!(config.config_text.contains("type = s3"));
}

#[test]
fn validate_config_accepts_the_target_remote() {
    assert!(validate_config(OVH_CONFIG, "ovhcloud-rbx").is_ok());
}

#[test]
fn validate_config_rejects_an_unknown_remote_name() {
    let err = validate_config(OVH_CONFIG, "typo").unwrap_err().to_string();
    assert!(err.contains("typo"), "unexpected error: {err}");
    assert!(err.contains("ovhcloud-rbx"), "error should list the available remotes: {err}");
}

#[test]
fn validate_config_rejects_local_backend() {
    let cfg = "[disk]\ntype = local\n";
    let err = validate_config(cfg, "disk").unwrap_err().to_string();
    assert!(err.contains("local"), "unexpected error: {err}");
}

#[test]
fn validate_config_rejects_alias_backend() {
    let cfg = "[shortcut]\ntype = alias\nremote = other:path\n";
    let err = validate_config(cfg, "shortcut").unwrap_err().to_string();
    assert!(err.contains("alias"), "unexpected error: {err}");
}

#[test]
fn validate_config_rejects_a_blocked_backend_in_a_chained_section() {
    // The target remote is fine, but it wraps a `local` remote. Checking only the
    // named section would let this through.
    let cfg = "[secret]\ntype = crypt\nremote = disk:vault\n\n[disk]\ntype = local\n";
    let err = validate_config(cfg, "secret").unwrap_err().to_string();
    assert!(err.contains("local"), "unexpected error: {err}");
    assert!(err.contains("disk"), "error should name the offending remote: {err}");
}

#[test]
fn validate_config_accepts_a_chained_crypt_over_s3() {
    let cfg = format!("[secret]\ntype = crypt\nremote = ovhcloud-rbx:bucket\n\n{OVH_CONFIG}");
    assert!(validate_config(&cfg, "secret").is_ok());
}

#[test]
fn remote_path_is_a_prefix_ahead_of_the_backup_folder() {
    // `remotePath` points at storage that may hold other things; every backup
    // lands under its own `backups/` subtree beneath it.
    assert_eq!(
        remote_target("ovhcloud-rbx", "my-bucket", "backups/2026-09-09/x.tar.gz"),
        "ovhcloud-rbx:my-bucket/backups/2026-09-09/x.tar.gz"
    );

    // Deeper prefixes nest the same way.
    assert_eq!(
        remote_target("ovhcloud-rbx", "my-bucket/portabase", "backups/2026-09-09/x.tar.gz"),
        "ovhcloud-rbx:my-bucket/portabase/backups/2026-09-09/x.tar.gz"
    );
}

#[test]
fn remote_target_trims_surrounding_slashes_and_whitespace() {
    assert_eq!(
        remote_target("r", "  /my-bucket/  ", "a/b.bin"),
        "r:my-bucket/a/b.bin"
    );
}

#[test]
fn remote_target_handles_an_empty_remote_path() {
    assert_eq!(remote_target("r", "", "a/b.bin"), "r:a/b.bin");
    assert_eq!(remote_target("r", "   ", "a/b.bin"), "r:a/b.bin");
}

#[test]
fn an_empty_remote_path_falls_back_to_the_global_backup_folder() {
    // remotePath is optional. When it is empty the destination comes entirely
    // from `full_file_path`, which the dashboard drives with
    // folderName = getBackupFolderName() (BACKUP_FOLDER_NAME, default "backups")
    // and which defaults to "backups" again on its own if that is absent.
    // No fallback code of our own — this test pins the composed result.
    let remote_file_path = full_file_path(&"x.tar.gz".to_string(), None);
    assert!(remote_file_path.starts_with("backups/"));

    assert_eq!(
        remote_target("ovhcloud-rbx", "", &remote_file_path),
        format!("ovhcloud-rbx:{remote_file_path}")
    );

    // Setting remotePath only prepends; the backups/<date>/ tail is unchanged.
    assert_eq!(
        remote_target("ovhcloud-rbx", "my-bucket", &remote_file_path),
        format!("ovhcloud-rbx:my-bucket/{remote_file_path}")
    );
}
