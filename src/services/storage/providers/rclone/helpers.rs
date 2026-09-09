use anyhow::{Result, bail};

/// Backends that would give a storage channel read/write access to the agent or
/// dashboard container filesystem. Rejected here as well as in the dashboard's
/// zod schema, because the agent receives this config over the wire.
const BLOCKED_BACKEND_TYPES: [&str; 2] = ["local", "alias"];

/// Section headers and their `type =` values, in file order.
fn sections(config_text: &str) -> Vec<(String, Option<String>)> {
    let mut out: Vec<(String, Option<String>)> = Vec::new();

    for line in config_text.lines() {
        let line = line.trim();

        if line.starts_with('[') && line.ends_with(']') && line.len() > 2 {
            out.push((line[1..line.len() - 1].trim().to_string(), None));
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            continue;
        };

        if key.trim().eq_ignore_ascii_case("type")
            && let Some(current) = out.last_mut()
            && current.1.is_none()
        {
            current.1 = Some(value.trim().to_ascii_lowercase());
        }
    }

    out
}

/// Rejects a config that names a missing remote or reaches a blocked backend.
/// Every section is checked, not only `remote_name` — a `crypt` remote can wrap
/// a `local` one, and checking only the named section would let that through.
pub fn validate_config(config_text: &str, remote_name: &str) -> Result<()> {
    let sections = sections(config_text);

    if sections.is_empty() {
        bail!("rclone config contains no remote sections");
    }

    for (name, backend) in &sections {
        let Some(backend) = backend else { continue };
        if BLOCKED_BACKEND_TYPES.contains(&backend.as_str()) {
            bail!("rclone backend type '{backend}' is not allowed (remote '{name}')");
        }
    }

    if !sections.iter().any(|(name, _)| name == remote_name) {
        let available: Vec<&str> = sections.iter().map(|(name, _)| name.as_str()).collect();
        bail!(
            "remote '{remote_name}' is not defined in the rclone config (available: {})",
            available.join(", ")
        );
    }

    Ok(())
}

/// `<remote>:<remote_path>/<remote_file_path>`, collapsing an empty path.
pub fn remote_target(remote_name: &str, remote_path: &str, remote_file_path: &str) -> String {
    let base = remote_path.trim().trim_matches('/');

    if base.is_empty() {
        format!("{remote_name}:{remote_file_path}")
    } else {
        format!("{remote_name}:{base}/{remote_file_path}")
    }
}
