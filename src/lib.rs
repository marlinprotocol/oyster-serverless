pub mod billing_handler;
pub mod cgroups;
pub mod handler;
pub mod model;
mod tests;
pub mod workerd;

use ethers::contract::abigen;
use ethers::providers::{Http, Provider};

abigen!(
    BillingContract,
    "src/contracts/billing_contract_abi.json",
    derives(serde::Serialize, serde::Deserialize)
);

type BillContract = BillingContract<Provider<Http>>;
