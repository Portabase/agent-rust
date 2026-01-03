mod settings;
mod tasks;
mod utils;
mod core;
mod services;
mod domain;

use tracing_subscriber;
use utils::redis_client;
use utils::task_manager::scheduler;
use crate::tasks::ping::ping_server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    tokio::join!(
        ping_server(),
        async {
            let conn = redis_client::redis_connection().await;
            scheduler::scheduler_loop(conn).await;
        }
    );
}
