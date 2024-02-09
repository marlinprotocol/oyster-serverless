use crate::model::AppState;

use actix_web::{web::Data, HttpResponse};
use anyhow::Context;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use serde_json::json;
use tiny_keccak::{Hasher, Keccak};

pub async fn billing_data(appstate: Data<AppState>) -> HttpResponse {
    let mut costs_gaurd = appstate.execution_costs.lock().unwrap();
    let costs_map = costs_gaurd.clone();

    let mut hash = [0u8; 32];
    let mut hasher_gaurd = appstate.billing_hasher.lock().unwrap();
    hasher_gaurd.clone().finalize(&mut hash);
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
    hasher_gaurd.clone_from(&Keccak::v256());

    HttpResponse::Ok().json(json!({
        "execution_costs": costs_map,
        "signature": signature,
    }))
}
