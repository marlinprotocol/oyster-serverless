use thiserror::Error;

use crate::cgroups::CgroupsError;

#[derive(Error, Debug)]
pub enum ServerlessError {
    #[error("failed to retrieve calldata")]
    CalldataRetrieve(#[from] reqwest::Error),
    #[error("tx not found")]
    TxNotFound,
    #[error("to field of transaction is not an address")]
    InvalidTxToType,
    #[error("to address {0} does not match expected {1}")]
    InvalidTxToValue(String, String),
    #[error("calldata field of transaction is not a string")]
    InvalidTxCalldataType,
    #[error("calldata is not a valid hex string")]
    BadCalldata(#[from] hex::FromHexError),
    #[error("failed to create code file")]
    CodeFileCreate(#[source] tokio::io::Error),
    #[error("failed to create config file")]
    ConfigFileCreate(#[source] tokio::io::Error),
    #[error("failed to execute workerd")]
    Execute(#[from] CgroupsError),
    #[error("failed to terminate workerd")]
    Terminate(#[source] tokio::io::Error),
    #[error("failed to delete code file")]
    CodeFileDelete(#[source] tokio::io::Error),
    #[error("failed to delete config file")]
    ConfigFileDelete(#[source] tokio::io::Error),
    #[error("failed to retrieve port from cgroup")]
    BadPort(#[source] std::num::ParseIntError),
    #[error("failed to retrieve number of code files")]
    CodeFileCount(#[source] tokio::io::Error),
    #[error("failed to update modified time of file")]
    UpdateModifiedTime(#[source] tokio::io::Error),
}
