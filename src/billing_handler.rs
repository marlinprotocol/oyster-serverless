use crate::model::AppState;

use actix_web::HttpResponse;
use actix_web::web::Data;
use anyhow::Context;
use ethers::types::U256;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use serde_json::json;
use tiny_keccak::{Hasher, Keccak};

pub async fn billing_data(appstate: Data<AppState>) -> HttpResponse {
    let mut execution_costs_gaurd = appstate.execution_costs.lock().unwrap();

    if execution_costs_gaurd.is_empty() {
        return HttpResponse::BadRequest().body("No billing data available");
    }

    let txhashes: Vec<[u8; 32]> = execution_costs_gaurd
        .keys()
        .cloned()
        .map(|txhash| {
            let mut bytes32_txhash = [0u8; 32];
            hex::decode_to_slice(&txhash[2..], &mut bytes32_txhash).unwrap();
            bytes32_txhash
        }).collect();
    let amounts: Vec<U256> = execution_costs_gaurd
        .values()
        .cloned()
        .map(|amount| U256::from(amount))
        .collect();

    let mut hasher = Keccak::v256();
    hasher.update(b"|txhashes|");
    hasher.update(txhashes
        .iter()
        .flat_map(|bytes32_txhash| bytes32_txhash.iter().cloned())
        .collect::<Vec<u8>>()
        .as_slice());
    hasher.update(b"|amounts|");
    hasher.update(amounts
        .iter()
        .flat_map(|amount| {
            let mut bytes_amount = [0u8; 32];
            amount.to_big_endian(&mut bytes_amount);
            bytes_amount
        }).collect::<Vec<u8>>()
        .as_slice());

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let sign = appstate
        .signer
        .sign_prehash_recoverable(&hash)
        .context("Failed to sign billing data");
    if sign.is_err() {
        return HttpResponse::InternalServerError().body(format!("{:?}", sign.unwrap_err()));
    }
    let (rs, v) = sign.unwrap();
    let signature = hex::encode(rs.to_bytes().append(27 + v.to_byte()).as_slice());

    execution_costs_gaurd.clear();

    HttpResponse::Ok().json(json!({
        "txhashes": txhashes,
        "amounts": amounts,
        "signature": signature,
    }))
}
