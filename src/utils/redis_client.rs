// use redis::{Client, Connection};
// use crate::settings::CONFIG;
//
// pub fn redis_connection() -> Connection {
//     let client = Client::open(CONFIG.redis_url.clone()).expect("Invalid Redis URL");
//     client.get_connection().expect("Failed to connect to Redis")
//
// }


use redis::{aio::MultiplexedConnection, Client};
use crate::settings::CONFIG;

pub async fn redis_connection() -> MultiplexedConnection {
    let client = Client::open(CONFIG.redis_url.clone())
        .expect("Invalid Redis URL");

    client
        .get_multiplexed_async_connection()
        .await
        .expect("Failed to connect to Redis")
}
