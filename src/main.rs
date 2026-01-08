mod core;
mod domain;
mod services;
mod settings;
mod tasks;
mod utils;

use crate::tasks::ping::ping_server;
use tracing_subscriber;
use utils::redis_client;
use utils::task_manager::scheduler;
use crate::utils::locks::FileLock;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Remove all locks on startup
    if let Err(e) = FileLock::clean_startup().await {
        eprintln!("Failed to clean locks on startup: {:?}", e);
    }

    tokio::join!(ping_server(), async {
        let conn = redis_client::redis_connection().await;
        scheduler::scheduler_loop(conn).await;
    });
}
