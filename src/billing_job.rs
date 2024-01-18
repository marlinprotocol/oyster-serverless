use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use actix_web::web;
use tiny_keccak::{Hasher, Keccak};
use crate::model::AppState;
use ethers::prelude::*;
use ethers::core::abi::Abi;
use k256::elliptic_curve::generic_array::sequence::Lengthen;

pub async fn billing_scheduler(appstate: web::Data<AppState>,) -> Result<String, anyhow::Error> {
    let abi_file_path = "src/contract_abi.json";          
    let mut abi_json = String::new();
    let mut file = std::fs::File::open(abi_file_path)?;
    file.read_to_string(&mut abi_json)?;
    let abi = serde_json::from_str::<Abi>(&abi_json)?;

    let provider = Provider::<Http>::try_from(&appstate.rpc)?.interval(Duration::from_millis(1000));
    let wallet: LocalWallet = appstate.operator_key.parse()?;

    let client = SignerMiddleware::new(provider, wallet);
    let contract = Contract::new(appstate.contract.as_str().parse::<Address>()?, abi, Arc::new(client));

    let mut tx_hashes = Vec::new();
    let mut amounts = Vec::new();
    for (key, value) in appstate.service_costs.lock().unwrap().iter() {
        tx_hashes.push(key.to_owned());
        amounts.push(value.to_owned());
    }

    let mut hash = [0u8; 32];
    let hasher = appstate.hasher.lock().unwrap().clone();
    hasher.finalize(&mut hash);
    let (rs, v) = appstate.signer.sign_prehash_recoverable(&hash)?;
    let signature = rs.to_bytes().append(27 + v.to_byte());
    
    let tx = contract.method::<_, H256>("settle", (tx_hashes, amounts, hex::encode(signature.as_slice())))?;
    let pending_tx = tx.send().await?;
    let receipt = pending_tx.confirmations(7).await?;

    appstate.service_costs.lock().unwrap().clear();
    appstate.hasher.lock().unwrap().clone_from(&Keccak::v256());

    Ok(receipt.unwrap().transaction_hash.to_string())
}