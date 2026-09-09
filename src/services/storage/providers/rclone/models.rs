use serde::{Deserialize, Serialize};

/// Deserialized from `DatabaseStorage.config`. Keys arrive camelCase from the
/// dashboard and are converted by `deserialize_snake_case` before this struct
/// sees them, so no serde rename is needed — same as `S3ProviderConfig`.
#[derive(Debug, Deserialize, Serialize)]
pub struct RcloneProviderConfig {
    /// The raw rclone config file the user pasted. May hold several sections.
    pub config_text: String,
    /// Which section of `config_text` is the upload target.
    pub remote_name: String,
    /// Optional prefix inside the remote, e.g. `my-bucket`. May be empty.
    /// `backups/<date>/<file>` is appended to it by `full_file_path`.
    pub remote_path: String,
}
