use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize)]
pub struct SystemInfoResposne {
    pub free_memory: u64,
    pub running_workerd_processes: usize,
}

#[derive(Serialize)]
pub struct JsonResponse {
    pub status: String,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Serialize, Deserialize)]
pub struct WorkerdDataResponse {
    pub execution_time: String,
    pub memory_usage: u64,
    pub user_address: String,
}
