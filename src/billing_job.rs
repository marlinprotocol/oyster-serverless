use std::sync::Arc;
use std::time::Duration;

use crate::model::AppState;

use actix_web::web::Data;
use anyhow::{anyhow, Context, Error};
use ethers::prelude::*;
use k256::elliptic_curve::generic_array::sequence::Lengthen;
use tiny_keccak::{Hasher, Keccak};

pub async fn billing_scheduler(appstate: Data<AppState>,) -> Result<String, Error> {
    let provider = Provider::<Http>::try_from(&appstate.rpc)
        .context("Unable to connect to the RPC")?
        .interval(Duration::from_millis(1000));
    let wallet: LocalWallet = appstate.operator_wallet_key
        .parse()
        .context("Unable to parse operator private key")?;

    let client = SignerMiddleware::new(provider, wallet);
    let contract = Contract::new(
        appstate.contract.as_str()
                    .parse::<Address>()
                    .context("Unable to parse contract address")?, 
        appstate.abi.to_owned(), 
        Arc::new(client));

    let mut costs_gaurd = appstate.execution_costs.lock().await;
    let mut tx_hashes = Vec::new();
    let mut amounts = Vec::new();
    for (tx_hash, amount) in costs_gaurd.clone().iter() {
        let mut bytes32_tx_hash = [0u8; 32];
        hex::decode_to_slice(&tx_hash[2..], &mut bytes32_tx_hash)
            .context("failed to decode tx hash to bytes")?;
        tx_hashes.push(bytes32_tx_hash);

        amounts.push(U256::from(amount.to_owned()));
    }

    let mut hash = [0u8; 32];
    let mut hasher_gaurd = appstate.billing_hasher.lock().await;
    hasher_gaurd.clone().finalize(&mut hash);
    let (rs, v) = appstate.signer
        .sign_prehash_recoverable(&hash)
        .context("failed to sign requests")?;
    let signature = hex::encode(rs
        .to_bytes()
        .append(27 + v.to_byte())
        .as_slice());
    
    let tx_request = contract.method::<_, H256>(
        "settle", 
        (tx_hashes, amounts, signature.as_bytes().to_vec()))
        .context("failed to build transaction request for billing")?;
    let pending_tx = tx_request
        .send()
        .await
        .context("Error while sending the billing transaction")?;
    
    let tx_receipt = pending_tx
        .confirmations(7)
        .await
        .context("failed to receive confirmation for billing")?;
    let tx_hash = tx_receipt
        .ok_or(anyhow!("Failed to parse transaction receipt!"))?
        .transaction_hash
        .to_string();

    costs_gaurd.clear();
    hasher_gaurd.clone_from(&Keccak::v256());

    Ok(tx_hash)    
}