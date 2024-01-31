use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, Mutex};

use crate::cgroups::Cgroups;

use tiny_keccak::Keccak;

pub struct AppState {
    pub cgroups: Mutex<Cgroups>,
    // IMPORTANT: we use Relaxed ordering here since we do not need to synchronize any memory
    // not even with reads/writes to the same atomic (we just serve a few more requests at worst)
    // be very careful adding more operations associated with the draining state
    pub running: AtomicBool,
    pub runtime_path: String,
    pub rpc: String,
    pub contract: String,
    pub billing_contract: String,
    pub signer: k256::ecdsa::SigningKey,
    pub execution_costs: Mutex<HashMap<String, u128>>,
    pub billing_hasher: Mutex<Keccak>,
}
