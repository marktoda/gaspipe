use crate::fork::spawn_fork;
use anvil_core::eth::transaction::EthTransactionRequest;
use anyhow::{anyhow, Result};
use ethers::types::{Address, Bytes, TransactionReceipt, U256};
use rocket::serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Transaction {
    pub from: Address,
    pub to: Address,
    pub data: Bytes,
    pub value: U256,
}

pub async fn execute(
    rpc_url: &str,
    transactions: Vec<Transaction>,
) -> Result<Vec<TransactionReceipt>> {
    let backend = spawn_fork(rpc_url).await?;
    let mut results: Vec<TransactionReceipt> = Vec::new();
    for tx in transactions {
        let request = EthTransactionRequest {
            from: Some(tx.from),
            to: Some(tx.to),
            data: Some(tx.data.clone()),
            value: Some(tx.value),
            ..Default::default()
        };
        let hash = backend.eth_send_unsigned_transaction(request).await?;
        backend.mine_one().await;
        let receipt = backend
            .transaction_receipt(hash)
            .await?
            .ok_or(anyhow!("Transaction not found"))?;
        results.push(receipt);
    }
    Ok(results)
}
