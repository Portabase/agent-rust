use std::path::{Path, PathBuf};
use std::process::Command;
use crate::domain::factory::Database;
use crate::services::config::DatabaseConfig;

pub struct PostgresDatabase {
    cfg: DatabaseConfig,
}

impl PostgresDatabase {
    pub fn new(cfg: DatabaseConfig) -> Self {
        Self { cfg }
    }
}

#[async_trait::async_trait]
impl Database for PostgresDatabase {
    fn file_extension(&self) -> &'static str {
        ".dump"
    }

    async fn ping(&self) -> anyhow::Result<bool> {
        let url = format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.cfg.username, self.cfg.password, self.cfg.host, self.cfg.port, self.cfg.database
        );
        let status = Command::new("pg_isready").arg("--dbname").arg(&url).status()?;
        Ok(status.success())
    }

    async fn backup(&self, backup_dir: &Path) -> anyhow::Result<PathBuf> {
        let file_path = backup_dir.join(format!("{}{}", self.cfg.generated_id, self.file_extension()));
        let url = format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.cfg.username, self.cfg.password, self.cfg.host, self.cfg.port, self.cfg.database
        );

        let status = Command::new("pg_dump")
            .arg("--dbname")
            .arg(url)
            .arg("-Fc")
            .arg("-f")
            .arg(&file_path)
            .arg("-v")
            .arg("--compress=3")
            .status()?;

        if !status.success() {
            anyhow::bail!("Postgres backup failed for {}", self.cfg.name);
        }

        Ok(file_path)
    }

    async fn restore(&self, restore_file: &Path) -> anyhow::Result<()> {
        let url = format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.cfg.username, self.cfg.password, self.cfg.host, self.cfg.port, "postgres"
        );

        // Terminate connections
        let terminate_cmd = format!(
            "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='{}' AND pid<>pg_backend_pid();",
            self.cfg.database
        );
        Command::new("psql")
            .arg("-U").arg(&self.cfg.username)
            .arg("-d").arg("postgres")
            .arg("-h").arg(&self.cfg.host)
            .arg("-p").arg(self.cfg.port.to_string())
            .arg("-c").arg(terminate_cmd)
            .env("PGPASSWORD", &self.cfg.password)
            .status()?;

        // Restore
        let status = Command::new("pg_restore")
            .arg("--no-owner")
            .arg("--no-privileges")
            .arg("--clean")
            .arg("--if-exists")
            .arg("--create")
            .arg("--dbname").arg(url)
            .arg("-v")
            .arg(restore_file)
            .env("PGPASSWORD", &self.cfg.password)
            .status()?;

        if !status.success() {
            anyhow::bail!("Postgres restore failed for {}", self.cfg.name);
        }

        Ok(())
    }
}

