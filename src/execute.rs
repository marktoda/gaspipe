use crate::fork::spawn_fork;
use anyhow::Result;
use ethers::types::{Address, Bytes, U256};
use rocket::serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Transaction {
    pub from: Address,
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct GasEstimate {
    pub gas: u64,
    pub reverted: bool,
}

pub async fn execute(rpc_url: &str, transactions: Vec<Transaction>) -> Result<Vec<GasEstimate>> {
    let mut backend = spawn_fork(rpc_url).await;
    let mut results: Vec<GasEstimate> = Vec::new();
    for tx in transactions {
        let receipt = backend
            .call_raw_committing(tx.from, tx.to, tx.data.0, tx.value)
            .map_err(|_| anyhow::anyhow!("Failed to call"))?;
        results.push(GasEstimate {
            gas: receipt.gas,
            reverted: receipt.reverted,
        });
    }
    Ok(results)
}
