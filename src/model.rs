use serde::Deserialize;
use std::collections::HashMap;
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct RequestBody {
    #[validate(length(min = 1), required)]
    pub tx_hash: Option<String>,
    pub input: Option<HashMap<String, serde_json::Value>>,
}
