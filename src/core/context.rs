use crate::settings::CONFIG;
use crate::utils::edge_key::{EdgeKey, EdgeKeyError, decode_edge_key};
use tracing::{debug, error, info};

#[derive(Debug)]
pub struct Context {
    #[allow(dead_code)]
    pub edge_key: EdgeKey,
}

impl Context {
    pub fn new() -> Self {
        let key = &CONFIG.edge_key;

        if key.is_empty() {
            error!("EDGE_KEY missing");
            panic!("EDGE_KEY missing");
        }

        let edge_key = match decode_edge_key(key) {
            Ok(k) => {
                debug!("EDGE_KEY decoded successfully");
                info!("EDGE_KEY server_url: {}", k.server_url);
                info!("EDGE_KEY agent_id: {}", k.agent_id);
                k
            }
            Err(e) => {
                match e {
                    EdgeKeyError::Base64Error(_) => error!("Base64 decoding error"),
                    EdgeKeyError::JsonError(_) => error!("JSON parsing error"),
                    EdgeKeyError::InvalidKey => error!("Invalid EDGE_KEY"),
                }
                panic!("Cannot initialize AgentContext due to invalid EDGE_KEY");
            }
        };

        Context { edge_key }
    }
}
