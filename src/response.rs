use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct JsonResponse {
    pub status: String,
    pub message: String,
    pub data: Option<Value>,
}