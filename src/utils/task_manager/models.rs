use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicTask {
    pub task: String,
    pub cron: String,
    pub args: Vec<String>,
    pub enabled: bool,
}
