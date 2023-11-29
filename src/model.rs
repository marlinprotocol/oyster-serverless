use crate::cgroups::Cgroups;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Mutex};
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct RequestBody {
    pub input: Option<HashMap<String, serde_json::Value>>,
}

pub struct AppState {
    pub cgroups: Mutex<Cgroups>,
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    pub running: AtomicBool,
    pub runtime_path: String,
}
