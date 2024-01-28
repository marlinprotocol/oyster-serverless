use std::sync::Arc;
use std::time::Duration;

use actix_web::web;
use anyhow::{anyhow, Context};
use tiny_keccak::{Hasher, Keccak};
use crate::model::AppState;
use ethers::prelude::*;
use k256::elliptic_curve::generic_array::sequence::Lengthen;

pub async fn billing_scheduler(appstate: web::Data<AppState>,) -> Result<String, anyhow::Error> {
    let provider = Provider::<Http>::try_from(&appstate.rpc)
        .context("Unable to connect to the RPC")?
        .interval(Duration::from_millis(1000));
    let wallet: LocalWallet = appstate.operator_key
        .parse()
        .context("Unable to parse operator private key")?;

    let client = SignerMiddleware::new(provider, wallet);
    let contract = Contract::new(
        appstate.contract.as_str()
                    .parse::<Address>()
                    .context("Unable to parse contract address")?, 
        appstate.abi.to_owned(), 
        Arc::new(client));

    let mut map_gaurd = appstate.service_costs.lock().await;
    let mut tx_hashes = Vec::new();
    let mut amounts = Vec::new();
    for (key, value) in map_gaurd.clone().iter() {
        let mut bytes32_tx_hash = [0u8; 32];
        hex::decode_to_slice(&key[2..], &mut bytes32_tx_hash).context("failed to decode tx hash to bytes")?;
        tx_hashes.push(bytes32_tx_hash);

        amounts.push(U256::from(value.to_owned()));
    }

    let mut hash = [0u8; 32];
    let mut hasher_gaurd = appstate.hasher.lock().await;
    hasher_gaurd.clone().finalize(&mut hash);
    let (rs, v) = appstate.signer
        .sign_prehash_recoverable(&hash)
        .context("failed to sign requests")?;
    let signature = hex::encode(rs
        .to_bytes()
        .append(27 + v.to_byte())
        .as_slice());
    
    let tx = contract.method::<_, H256>(
        "settle", 
        (tx_hashes, amounts, signature.as_bytes().to_vec()))
        .context("failed to build transaction request for billing")?;
    let pending_tx = tx
        .send()
        .await
        .context("Error while sending the billing transaction")?;
    let receipt = pending_tx
        .confirmations(7)
        .await
        .context("failed to receive confirmation for billing")?;
    let tx_hash = receipt
        .ok_or(anyhow!("Failed to parse transaction receipt!"))?
        .transaction_hash
        .to_string();

    map_gaurd.clear();
    hasher_gaurd.clone_from(&Keccak::v256());

    Ok(tx_hash)    
}