use crate::cgroups::Cgroups;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Mutex};
use validator::Validate;

fn validate_hash(value: &str) -> Result<(), validator::ValidationError> {
    if &value[0..2] == "0x" && value[2..].as_bytes().iter().all(|x| x.is_ascii_hexdigit()) {
        Ok(())
    } else {
        Err(validator::ValidationError::new("invalid hex string"))
    }
}

#[derive(Debug, Validate, Deserialize)]
pub struct RequestBody {
    #[validate(length(equal = 66), custom = "validate_hash")]
    pub tx_hash: String,
    pub input: Option<HashMap<String, serde_json::Value>>,
}

pub struct AppState {
    pub cgroups: Mutex<Cgroups>,
    pub cgroup_list: Vec<String>,
    pub cgroup_version: u8,
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    pub running: AtomicBool,
    pub runtime_path: String,
}
