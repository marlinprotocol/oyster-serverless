use crate::model::AppState;

use actix_web::{web::Data, HttpResponse};
use anyhow::Context;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use serde_json::json;
use tiny_keccak::{Hasher, Keccak};

pub async fn billing_data(appstate: Data<AppState>) -> HttpResponse {
    let mut costs_gaurd = appstate.execution_costs.lock().unwrap();

    if costs_gaurd.is_empty() {
        return HttpResponse::BadRequest().body("Bill is empty");
    }

    let tx_hashes: Vec<String> = costs_gaurd.keys().cloned().collect();
    let amounts: Vec<u128> = costs_gaurd.values().cloned().collect();

    let mut hasher = Keccak::v256();
    hasher.update(b"|txhashes|");
    hasher.update(
        tx_hashes
            .iter()
            .flat_map(|txhash| txhash.as_bytes().to_vec())
            .collect::<Vec<u8>>()
            .as_slice(),
    );
    hasher.update(b"|amounts|");
    hasher.update(
        amounts
            .iter()
            .flat_map(|amount| amount.to_be_bytes().to_vec())
            .collect::<Vec<u8>>()
            .as_slice(),
    );

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let sign = appstate
        .signer
        .sign_prehash_recoverable(&hash)
        .context("Failed to sign requests data");
    if sign.is_err() {
        return HttpResponse::InternalServerError().body(format!("{:?}", sign.unwrap_err()));
    }
    let (rs, v) = sign.unwrap();
    let signature = hex::encode(rs.to_bytes().append(27 + v.to_byte()).as_slice());

    costs_gaurd.clear();

    HttpResponse::Ok().json(json!({
        "txhashes": tx_hashes,
        "amounts": amounts,
        "signature": signature,
    }))
}
