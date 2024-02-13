use crate::model::AppState;

use actix_web::web::{Data, Json};
use actix_web::{get, post, HttpResponse, Responder};
use anyhow::Context;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use serde_json::json;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, serde::Deserialize)]
pub struct SigningData {
    nonce: String,
    txhashes: Vec<String>,
}

#[get("/billing/inspect")]
pub async fn get_bill(appstate: Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(json!({
        "bill": appstate.execution_costs.lock().unwrap().clone(),
    }))
}

#[post("/billing/export")]
pub async fn sign_data(appstate: Data<AppState>, data: Json<SigningData>) -> impl Responder {
    let signing_data = data.into_inner();
    if signing_data.nonce.is_empty() {
        return HttpResponse::BadRequest().body("Nonce must not be empty");
    }

    if signing_data.txhashes.is_empty() {
        return HttpResponse::BadRequest().body("List of tx hashes must not be empty");
    }

    let mut bytes32_nonce = [0u8; 32];
    if let Err(err) = hex::decode_to_slice(&signing_data.nonce, &mut bytes32_nonce) {
        return HttpResponse::BadRequest()
            .body(format!("Error decoding nonce into 32 bytes: {}", err));
    }

    let mut signed_data: Vec<u8> = bytes32_nonce.to_vec();
    for txhash in signing_data.txhashes {
        if let Some(cost) = appstate.execution_costs.lock().unwrap().remove(&txhash) {
            let mut bytes32_txhash = [0u8; 32];
            if let Err(err) = hex::decode_to_slice(&txhash[2..], &mut bytes32_txhash) {
                return HttpResponse::InternalServerError().body(format!(
                    "Error decoding transaction hash into 32 bytes: {}",
                    err
                ));
            }

            signed_data.append(&mut bytes32_txhash.to_vec());
            signed_data.append(&mut cost.to_be_bytes().to_vec());
        } else {
            return HttpResponse::BadRequest().body(format!(
                "{} tx hash doesn't exist in the current bill",
                txhash
            ));
        }
    }

    let mut hasher = Keccak::v256();
    hasher.update(&signed_data);

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
    let signed_data = hex::encode(signed_data.as_slice());

    HttpResponse::Ok().json(json!({
        "signed_data": signed_data,
        "signature": signature,
    }))
}
