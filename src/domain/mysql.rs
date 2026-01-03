use crate::domain::factory::Database;
use crate::services::config::DatabaseConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MySQLDatabase {
    cfg: DatabaseConfig,
}

impl MySQLDatabase {
    pub fn new(cfg: DatabaseConfig) -> Self {
        Self { cfg }
    }

    fn build_env(&self) -> HashMap<String, String> {
        let mut envs = std::env::vars().collect::<HashMap<_, _>>();
        envs.insert("MYSQL_PWD".to_string(), self.cfg.password.clone());
        envs
    }
}

#[async_trait::async_trait]
impl Database for MySQLDatabase {
    fn file_extension(&self) -> &'static str {
        ".sql"
    }

    async fn ping(&self) -> Result<bool> {
        let output = Command::new("mysqladmin")
            .arg("--host")
            .arg(&self.cfg.host)
            .arg("--port")
            .arg(self.cfg.port.to_string())
            .arg("--user")
            .arg(&self.cfg.username)
            .arg("ping")
            .envs(self.build_env())
            .output()
            .with_context(|| format!("Failed to ping MySQL server {}", self.cfg.name))?;

        Ok(output.status.success())
    }

    async fn backup(&self, backup_dir: &Path) -> Result<PathBuf> {
        let file_path = backup_dir.join(format!(
            "{}{}",
            self.cfg.generated_id,
            self.file_extension()
        ));

        let output = Command::new("mysqldump")
            .arg("--host")
            .arg(&self.cfg.host)
            .arg("--port")
            .arg(self.cfg.port.to_string())
            .arg("--user")
            .arg(&self.cfg.username)
            .arg("--routines")
            .arg("--events")
            .arg("--triggers")
            .arg("--verbose")
            .arg("--single-transaction")
            .arg("--quick")
            .arg("--add-drop-database")
            .arg("--databases")
            .arg(&self.cfg.database)
            .arg("-r")
            .arg(&file_path)
            .envs(self.build_env())
            .output()
            .with_context(|| format!("Failed to run mysqldump for {}", self.cfg.name))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("MySQL backup failed for {}: {}", self.cfg.name, stderr);
        }

        Ok(file_path)
    }

    async fn restore(&self, restore_file: &Path) -> Result<()> {
        let sql_content = tokio::fs::read_to_string(restore_file)
            .await
            .with_context(|| format!("Failed to read restore file {}", restore_file.display()))?;

        let drop_create_cmd = format!(
            "DROP DATABASE IF EXISTS {0}; CREATE DATABASE {0};",
            self.cfg.database
        );

        let drop_status = Command::new("mysql")
            .arg("--host")
            .arg(&self.cfg.host)
            .arg("--port")
            .arg(self.cfg.port.to_string())
            .arg("--user")
            .arg(&self.cfg.username)
            .arg("-e")
            .arg(&drop_create_cmd)
            .env("MYSQL_PWD", &self.cfg.password)
            .status()
            .with_context(|| format!("Failed to drop/recreate database {}", self.cfg.name))?;

        if !drop_status.success() {
            anyhow::bail!("Failed to drop/recreate database {}", self.cfg.name);
        }

        let mut child = Command::new("mysql")
            .arg("--host")
            .arg(&self.cfg.host)
            .arg("--port")
            .arg(self.cfg.port.to_string())
            .arg("--user")
            .arg(&self.cfg.username)
            .arg(&self.cfg.database)
            .env("MYSQL_PWD", &self.cfg.password)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to start mysql restore for {}", self.cfg.name))?;

        let mut stdin = child.stdin.take().context("Failed to open child stdin")?;
        stdin.write_all(sql_content.as_bytes())?;
        stdin.flush()?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .with_context(|| format!("Failed to complete mysql restore for {}", self.cfg.name))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("MySQL restore failed for {}: {}", self.cfg.name, stderr);
        }

        Ok(())
    }
}
