use crate::model::AppState;

use actix_web::web::Data;
use actix_web::HttpResponse;
use anyhow::Context;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use serde_json::json;
use tiny_keccak::{Hasher, Keccak};

pub async fn billing_data(appstate: Data<AppState>) -> HttpResponse {
    let mut execution_costs_gaurd = appstate.execution_costs.lock().unwrap();

    if execution_costs_gaurd.is_empty() {
        return HttpResponse::BadRequest().body("No billing data available");
    }

    let mut billing_data: Vec<u8> = Vec::new();

    for (tx_hash, cost) in execution_costs_gaurd.iter() {
        let mut bytes32_tx_hash = [0u8; 32];
        if let Err(err) = hex::decode_to_slice(&tx_hash[2..], &mut bytes32_tx_hash) {
            return HttpResponse::InternalServerError()
                .body(format!("Error decoding transaction hash: {:?}", err));
        }
        
        billing_data.append(&mut bytes32_tx_hash.to_vec());
        billing_data.append(&mut cost.to_be_bytes().to_vec());
    }

    let mut hasher = Keccak::v256();
    hasher.update(&billing_data);

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let sign = appstate
        .signer
        .sign_prehash_recoverable(&hash)
        .context("Failed to sign billing data");
    if sign.is_err() {
        return HttpResponse::InternalServerError().body(format!("{}", sign.unwrap_err()));
    }
    let (rs, v) = sign.unwrap();
    let signature = hex::encode(rs.to_bytes().append(27 + v.to_byte()).as_slice());

    let billing_data = hex::encode(billing_data.as_slice());
    execution_costs_gaurd.clear();

    HttpResponse::Ok().json(json!({
        "billing_data": billing_data,
        "signature": signature,
    }))
}
