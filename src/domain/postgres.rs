#![allow(dead_code)]

use crate::domain::factory::Database;
use crate::services::config::DatabaseConfig;
use anyhow::Result;
use async_trait::async_trait;
use flate2::Compression;
use flate2::write::GzEncoder;
use log::info;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio_postgres::{Client, NoTls};
use tracing::debug;

#[derive(Clone, Copy)]
pub enum PostgresDumpFormat {
    Fc, // legacy
    Fd, // directory format
}

pub struct PostgresDatabase {
    cfg: DatabaseConfig,
    format: PostgresDumpFormat,
}

impl PostgresDatabase {
    pub fn new(cfg: DatabaseConfig, format: PostgresDumpFormat) -> Self {
        Self { cfg, format }
    }

    pub fn detect_format_from_file(restore_file: &Path) -> PostgresDumpFormat {
        match restore_file.extension().and_then(|e| e.to_str()) {
            Some("dump") => PostgresDumpFormat::Fc,
            Some("gz") => PostgresDumpFormat::Fd,
            // Some("tar.gz") => PostgresDumpFormat::Fd,
            _ => PostgresDumpFormat::Fc,
        }
    }

    pub async fn detect_format_from_size(cfg: &DatabaseConfig) -> PostgresDumpFormat {
        info!(
            "Detecting database format {:?} - {:?}",
            cfg.name, cfg.generated_id
        );
        let client = match PostgresDatabase::connect(cfg).await {
            Ok(c) => c,
            Err(_) => return PostgresDumpFormat::Fc,
        };

        let row = match client
            .query_one("SELECT pg_database_size(current_database());", &[])
            .await
        {
            Ok(r) => r,
            Err(_) => return PostgresDumpFormat::Fc,
        };

        let size_bytes: i64 = row.get(0);
        info!("Size of database is {} bytes", size_bytes);

        // > 1 Go
        if size_bytes > 1_000_000_000 {
            info!("Using -Fd format");
            PostgresDumpFormat::Fd
        } else {
            info!("Using -Fc format");
            PostgresDumpFormat::Fc
        }
    }

    async fn connect(cfg: &DatabaseConfig) -> anyhow::Result<Client> {
        let dsn = format!(
            "host={} port={} user={} password={} dbname={}",
            cfg.host, cfg.port, cfg.username, cfg.password, cfg.database
        );

        let (client, connection) = tokio_postgres::connect(&dsn, NoTls).await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                tracing::error!("Postgres connection error: {}", e);
            }
        });

        Ok(client)
    }

    async fn get_postgres_server_version(cfg: &DatabaseConfig) -> anyhow::Result<String> {
        let client = Self::connect(cfg).await?;

        // Query the version string
        let row = client.query_one("SHOW server_version;", &[]).await?;
        let version_str: String = row.get(0);

        // Parse the major version
        let major_version: u32 = version_str
            .split('.')
            .next()
            .ok_or_else(|| anyhow::anyhow!("Cannot parse PostgreSQL version"))?
            .parse()
            .map_err(|_| anyhow::anyhow!("Failed to parse PostgreSQL major version"))?;

        if !(12..=18).contains(&major_version) {
            anyhow::bail!(
                "PostgreSQL version {} not supported, must be between 12 and 18",
                major_version
            );
        }

        Ok(version_str)
    }

    fn select_pg_path(version: &str) -> PathBuf {
        let major = version.split('.').next().unwrap_or("17"); // default to 17
        PathBuf::from(format!("/usr/lib/postgresql/{}/bin", major))
    }

    async fn terminate_connections(cfg: &DatabaseConfig) -> anyhow::Result<()> {
        let mut admin_cfg = cfg.clone();
        admin_cfg.database = "postgres".to_string();

        let client = Self::connect(&admin_cfg).await?;

        client
            .execute(
                "
        SELECT pg_terminate_backend(pid)
        FROM pg_stat_activity
        WHERE datname = $1
          AND pid <> pg_backend_pid();
        ",
                &[&cfg.database],
            )
            .await?;

        Ok(())
    }
}

#[async_trait]
impl Database for PostgresDatabase {
    fn file_extension(&self) -> &'static str {
        match self.format {
            PostgresDumpFormat::Fc => ".dump",
            PostgresDumpFormat::Fd => ".tar.gz",
        }
    }

    // async fn ping(&self) -> Result<bool> {
    //     let server_version = PostgresDatabase::get_postgres_server_version(&self.cfg).await?;
    //     let pg_path = PostgresDatabase::select_pg_path(&server_version.to_string());
    //     let pg_isready_path = format!("{}/pg_isready", pg_path.display());
    //
    //     debug!("Server version: {} -> {}", server_version, pg_isready_path);
    //
    //     let url = format!(
    //         "postgresql://{}:{}@{}:{}/{}",
    //         self.cfg.username, self.cfg.password, self.cfg.host, self.cfg.port, self.cfg.database
    //     );
    //
    //     let status = Command::new(pg_isready_path)
    //         .arg("--dbname")
    //         .arg(url)
    //         .status()
    //         .context("Failed to ping Postgres")?;
    //     Ok(status.success())
    // }
    async fn ping(&self) -> Result<bool> {
        Ok(Self::connect(&self.cfg).await.is_ok())
    }

    async fn backup(&self, backup_dir: &Path) -> Result<PathBuf> {
        let server_version = PostgresDatabase::get_postgres_server_version(&self.cfg).await?;
        let pg_path = PostgresDatabase::select_pg_path(&server_version.to_string());
        let pg_dump_path = format!("{}/pg_dump", pg_path.display());

        debug!("Server version: {} -> {}", server_version, pg_dump_path);

        match self.format {
            PostgresDumpFormat::Fc => {
                let file_path = backup_dir.join(format!(
                    "{}{}",
                    self.cfg.generated_id,
                    self.file_extension()
                ));
                let url = format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    self.cfg.username,
                    self.cfg.password,
                    self.cfg.host,
                    self.cfg.port,
                    self.cfg.database
                );
                let status = Command::new(pg_dump_path)
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
            PostgresDumpFormat::Fd => {
                // directory dump -> tar.gz
                let dump_dir = backup_dir.join(format!("{}_dir", self.cfg.generated_id));
                let tar_file = backup_dir.join(format!("{}.tar.gz", self.cfg.generated_id));
                std::fs::create_dir_all(&dump_dir)?;
                let url = format!(
                    "postgresql://{}:{}@{}:{}/{}",
                    self.cfg.username,
                    self.cfg.password,
                    self.cfg.host,
                    self.cfg.port,
                    self.cfg.database
                );
                let status = Command::new(pg_dump_path)
                    .arg("--dbname")
                    .arg(url)
                    .arg("-Fd")
                    .arg("-j")
                    .arg("4")
                    .arg("-f")
                    .arg(&dump_dir)
                    .arg("-v")
                    .status()?;
                if !status.success() {
                    anyhow::bail!("Postgres Fd backup failed for {}", self.cfg.name);
                }

                // Compression tar.gz
                let tar_gz = std::fs::File::create(&tar_file)?;
                let enc = GzEncoder::new(tar_gz, Compression::default());
                let mut tar = tar::Builder::new(enc);
                tar.append_dir_all(".", &dump_dir)?;
                tar.finish()?;
                Ok(tar_file)
            }
        }
    }

    async fn restore(&self, restore_file: &Path) -> Result<()> {
        info!("Restoring database \"{}\"", self.cfg.database);
        let server_version = PostgresDatabase::get_postgres_server_version(&self.cfg).await?;
        let pg_path = PostgresDatabase::select_pg_path(&server_version.to_string());
        let pg_restore_path = format!("{}/pg_restore", pg_path.display());

        debug!("Server version: {} -> {}", server_version, pg_restore_path);

        let url = format!(
            "postgresql://{}:{}@{}:{}/{}",
            self.cfg.username, self.cfg.password, self.cfg.host, self.cfg.port, "postgres"
        );

        PostgresDatabase::terminate_connections(&self.cfg).await?;

        match self.format {
            PostgresDumpFormat::Fc => {
                let status = Command::new(pg_restore_path)
                    .arg("--no-owner")
                    .arg("--no-privileges")
                    .arg("--clean")
                    .arg("--if-exists")
                    .arg("--create")
                    .arg("--dbname")
                    .arg(url)
                    .arg("-v")
                    .arg(restore_file)
                    .env("PGPASSWORD", &self.cfg.password)
                    .status()?;
                if !status.success() {
                    anyhow::bail!("Postgres restore failed for {}", self.cfg.name);
                }
            }
            PostgresDumpFormat::Fd => {
                let tar_gz = std::fs::File::open(restore_file)?;
                let dec = flate2::read::GzDecoder::new(tar_gz);
                let mut archive = tar::Archive::new(dec);
                let tmp_dir = tempfile::TempDir::new()?;
                archive.unpack(tmp_dir.path())?;

                let dump_dir = tmp_dir.path();

                info!("Restoring dump from {}", dump_dir.display());
                let status = Command::new(pg_restore_path)
                    .arg("--no-owner")
                    .arg("--no-privileges")
                    .arg("--clean")
                    .arg("--if-exists")
                    .arg("--create")
                    .arg("--dbname")
                    .arg(url)
                    .arg("-v")
                    .arg(dump_dir)
                    .env("PGPASSWORD", &self.cfg.password)
                    .status()?;
                if !status.success() {
                    anyhow::bail!("Postgres Fd restore failed for {}", self.cfg.name);
                }
            }
        }
        Ok(())
    }
}
