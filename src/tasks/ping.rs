use crate::core::agent::Agent;
use crate::core::context::Context;
use crate::settings::CONFIG;
use crate::utils::common::BackupMethod;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info};

pub async fn ping_server() {
    let ctx = Arc::new(Context::new());
    let mut agent = Agent::new(ctx.clone()).await;

    loop {
        info!("Ping server task started");

        if let Err(e) = agent.run(BackupMethod::Manual).await {
            error!(
                "An error occurred while executing task ping_server: {:?}",
                e
            );
        }

        tokio::time::sleep(Duration::from_secs(CONFIG.pooling as u64)).await;
    }
}
// use crate::core::agent::Agent;
// use crate::core::context::Context;
// use crate::utils::common::BackupMethod;
// use std::sync::Arc;
// use tokio::sync::Mutex;
// use tokio::time::{sleep, Duration};
// use tracing::{error};
//
// pub async fn ping_server() {
//
//
//     loop {
//         let ctx = Arc::new(Context::new());
//         let agent = Arc::new(Mutex::new(Agent::new(ctx.clone()).await));
//         let agent_clone = agent.clone();
//
//         tokio::spawn(async move {
//             let mut agent_locked = agent_clone.lock().await;
//             if let Err(e) = agent_locked.run(BackupMethod::Manual).await {
//                 error!("An error occurred while executing ping_server: {:?}", e);
//             }
//         });
//
//         sleep(Duration::from_secs(5)).await;
//     }
// }
