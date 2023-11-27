pub mod cgroups;
pub mod workerd;

use thiserror::Error;

#[derive(Error, Debug)]
enum ServerlessError {
    #[error("failed to retrieve calldata")]
    CalldataRetrieve(#[from] reqwest::Error),
    #[error("Tx not found")]
    TxNotFound,
    #[error("To field of transaction is not an address")]
    InvalidTxToType,
    #[error("To address {0} does not match expected {1}")]
    InvalidTxToValue(String, &'static str),
    #[error("Calldata field of transaction is not a string")]
    InvalidTxCalldataType,
    #[error("Calldata is not a valid hex string")]
    BadCalldata(#[from] hex::FromHexError),
    #[error("failed to create code file")]
    CodeFileCreate(#[from] tokio::io::Error),
}
