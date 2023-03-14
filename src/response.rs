use serde::{Serialize};
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
