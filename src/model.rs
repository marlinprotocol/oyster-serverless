use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Mutex;
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct RequestBody {
    #[validate(length(min = 1), required)]
    pub tx_hash: Option<String>,
    pub input: Option<HashMap<String, serde_json::Value>>,
}

pub struct AppState {
    pub cgroup_list: Vec<String>,
    pub cgroup_version: u8,
    pub running: Mutex<bool>,
    pub runtime_path: String,
}
