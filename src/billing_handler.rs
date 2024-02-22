use crate::model::AppState;

use actix_web::web::{Data, Json};
use actix_web::{get, post, HttpResponse, Responder};
use k256::ecdsa::SigningKey;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use serde_json::json;
use tiny_keccak::{Hasher, Keccak};

#[derive(Debug, serde::Deserialize)]
pub struct SigningData {
    nonce: String,
    tx_hashes: Vec<String>,
}

#[get("/billing/inspect")]
pub async fn inspect_bill(appstate: Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(json!({
        "bill": appstate.execution_costs.lock().unwrap().clone(),
    }))
}

#[get("/billing/latest")]
pub async fn get_last_bill_claim(appstate: Data<AppState>) -> impl Responder {
    let mut last_bill_claim_guard = appstate.last_bill_claim.lock().unwrap();
    let Some(bill_data_hex) = last_bill_claim_guard.0.clone() else {
        return HttpResponse::BadRequest().body("No bill claimed yet!");
    };

    if let Some(signature) = last_bill_claim_guard.1.clone() {
        return HttpResponse::Ok().json(json!({
            "bill_claim_data": bill_data_hex,
            "signature": signature,
        }));
    }

    let bill_claim_data = hex::decode(&bill_data_hex);
    let Ok(bill_claim_data) = bill_claim_data else {
        return HttpResponse::InternalServerError().body(format!(
            "Failed to decode claimed bill data: {}",
            bill_claim_data.unwrap_err()
        ));
    };

    let signature = sign_data(bill_claim_data.as_slice(), &appstate.signer).await;
    let Ok(signature) = signature else {
        return HttpResponse::InternalServerError().body(format!(
            "Failed to sign billing data: {}",
            signature.unwrap_err()
        ));
    };

    last_bill_claim_guard.1 = Some(signature.clone());

    HttpResponse::Ok().json(json!({
        "bill_claim_data": bill_data_hex,
        "signature": signature,
    }))
}

#[post("/billing/export")]
pub async fn export_bill(appstate: Data<AppState>, data: Json<SigningData>) -> impl Responder {
    let signing_data = data.into_inner();
    if signing_data.nonce.is_empty() {
        return HttpResponse::BadRequest().body("Nonce must not be empty!");
    }

    if signing_data.tx_hashes.is_empty() {
        return HttpResponse::BadRequest().body("List of transaction hashes must not be empty!");
    }

    let mut bytes32_nonce = [0u8; 32];
    if let Err(err) = hex::decode_to_slice(&signing_data.nonce, &mut bytes32_nonce) {
        return HttpResponse::BadRequest()
            .body(format!("Failed to decode nonce into 32 bytes: {}", err));
    }

    let mut bill_claim_data = bytes32_nonce.to_vec();
    for tx_hash in signing_data.tx_hashes {
        if let Some(cost) = appstate.execution_costs.lock().unwrap().remove(&tx_hash) {
            let mut bytes32_tx_hash = [0u8; 32];
            if let Err(_) = hex::decode_to_slice(&tx_hash[2..], &mut bytes32_tx_hash) {
                continue;
            }

            bill_claim_data.append(&mut bytes32_tx_hash.to_vec());
            bill_claim_data.append(&mut cost.to_be_bytes().to_vec());
        }
    }

    if bill_claim_data.len() == 32 {
        return HttpResponse::BadRequest()
            .body("Given transaction hashes are not present in billing data");
    }

    let signature = sign_data(bill_claim_data.as_slice(), &appstate.signer).await;
    let Ok(signature) = signature else {
        appstate
            .last_bill_claim
            .lock()
            .unwrap()
            .0
            .clone_from(&Some(hex::encode(bill_claim_data.as_slice())));

        return HttpResponse::InternalServerError().body(format!(
            "Failed to sign billing data: {}",
            signature.unwrap_err()
        ));
    };

    let bill_claim_data = hex::encode(bill_claim_data.as_slice());

    let mut last_bill_claim_guard = appstate.last_bill_claim.lock().unwrap();
    last_bill_claim_guard.0 = Some(bill_claim_data.clone());
    last_bill_claim_guard.1 = Some(signature.clone());

    HttpResponse::Ok().json(json!({
        "bill_claim_data": bill_claim_data,
        "signature": signature,
    }))
}

async fn sign_data(data: &[u8], signer: &SigningKey) -> Result<String, anyhow::Error> {
    let mut hasher = Keccak::v256();
    hasher.update(data);

    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);

    let (rs, v) = signer.sign_prehash_recoverable(&hash)?;
    let signature = hex::encode(rs.to_bytes().append(27 + v.to_byte()).as_slice());

    return Ok(signature);
}
