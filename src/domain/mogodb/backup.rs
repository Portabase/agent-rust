use crate::domain::mogodb::connection::select_mongo_path;
use crate::services::config::DatabaseConfig;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, error, info};

pub async fn run(
    cfg: DatabaseConfig,
    backup_dir: PathBuf,
    env: HashMap<String, String>,
    file_extension: &'static str,
) -> Result<PathBuf> {
    tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        debug!("Starting MongoDB backup for database {}", cfg.name);

        let file_path = backup_dir.join(format!("{}{}", cfg.generated_id, file_extension));

        let mongodump = select_mongo_path().join("mongodump");

        // let output = Command::new(mongodump)
        //     .arg("--host")
        //     .arg(&cfg.host)
        //     .arg("--port")
        //     .arg(cfg.port.to_string())
        //     .arg("--username")
        //     .arg(&cfg.username)
        //     .arg("--password")
        //     .arg(&cfg.password)
        //     .arg("--db")
        //     .arg(&cfg.database)
        //     // .arg("--authenticationDatabase")
        //     // .arg("admin")
        //     .arg("--archive")
        //     .arg(&file_path)
        //     .arg("--gzip")
        //     .envs(env)
        //     .output()
        //     .with_context(|| format!("Failed to run mongodump for {}", cfg.name))?;

        // let uri = format!(
        //     "mongodb://{}:{}@{}:{}/{}",
        //     cfg.username, cfg.password, cfg.host, cfg.port, cfg.database
        // );
        //
        // info!("MongoDB backup URL: {}", uri);
        //
        // let output = Command::new(mongodump)
        //     .arg("--uri")
        //     .arg(&uri)
        //     .arg("--archive")
        //     .arg(&file_path)
        //     .arg("--gzip")
        //     .envs(env)
        //     .output()
        //     .with_context(|| format!("Failed to run mongodump for {}", cfg.name))?;

        // let uri = format!(
        //     "mongodb://root:rootpassword@mongodb-auth:27017/testdbauth?authSource=admin"
        // );
        //
        // let output = Command::new(select_mongo_path().join(mongodump))
        //     .arg("--uri")
        //     .arg(&uri)
        //     .arg("--archive")
        //     .arg(&file_path)
        //     .arg("--gzip")
        //     // .envs(env)
        //     .output()
        //     .with_context(|| format!("Failed to run mongodump for {}", cfg.name))?;
        // let mut output = Command::new(mongodump);
        //
        // output.env_remove("MONGODB_URI");
        // output.env_remove("MONGO_URL");
        //
        // // let output = Command::new(select_mongo_path().join(mongodump))
        // output.arg("--host")
        //     .arg(&cfg.host)
        //     .arg("--port")
        //     .arg(cfg.port.to_string())
        //     .arg("--username")
        //     .arg(&cfg.username)
        //     .arg("--password")
        //     .arg(&cfg.password)
        //     .arg("--authenticationDatabase")
        //     .arg("admin")
        //     .arg("--db")
        //     .arg(&cfg.database)
        //     .arg("--archive")
        //     .arg(&file_path)
        //     .arg("--gzip")
        //     // .envs(env)
        //     .output()?;

        // let mut cmd = Command::new(mongodump);
        //
        // // Neutralisation explicite de toute URI Mongo héritée
        // cmd.env_remove("MONGODB_URI");
        // cmd.env_remove("MONGO_URL");
        //
        // // Commande mongodump en mode flags (une seule source de connexion)
        // cmd
        //     .arg("--host")
        //     .arg(&cfg.host)
        //     .arg("--port")
        //     .arg(cfg.port.to_string())
        //     .arg("--username")
        //     .arg(&cfg.username)
        //     .arg("--password")
        //     .arg(&cfg.password)
        //     .arg("--authenticationDatabase")
        //     .arg("admin")
        //     .arg("--db")
        //     .arg(&cfg.database)
        //     .arg("--archive")
        //     .arg(&file_path)
        //     .arg("--gzip")
        //     .envs(env);
        let uri = format!(
            "mongodb://{}:{}@{}:{}/{}?authSource=admin",
            cfg.username,
            cfg.password,
            cfg.host,
            cfg.port,
            cfg.database
        );

        let output = Command::new(mongodump)
            .arg(format!("--uri={}", uri))
            .arg(format!("--archive={}", file_path.display()))
            .output()
            .context("MongoDB backup failed")?;

        // let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("MongoDB backup failed for {}: {}", cfg.name, stderr);
            anyhow::bail!("MongoDB backup failed for {}: {}", cfg.name, stderr);
        }

        info!("MongoDB backup completed for {}", cfg.name);
        Ok(file_path)
    })
    .await?
}
