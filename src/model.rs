use serde::Deserialize;
use validator::Validate;

#[derive(Debug, Validate, Deserialize)]
pub struct RequestBody {
    #[validate(length(min = 1), required)]
    pub tx_hash: Option<String>,
}
