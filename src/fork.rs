use anvil::{
    eth::{
        backend::{
            db::Db,
            fork::{ClientFork, ClientForkConfig},
            genesis::GenesisConfig,
            mem,
            mem::fork_db::ForkedDatabase,
        },
        fees::FeeManager,
        miner::{Miner, MiningMode},
        pool::Pool,
        sign::{DevSigner, Signer as EthSigner},
        EthApi,
    },
    filter::Filters,
    NodeConfig,
};
use anyhow::{anyhow, Result};
use ethers::{
    providers::{Http, Middleware, Provider, RetryClient},
    signers::Signer,
    types::BlockNumber,
};
use foundry_evm::{
    executor::fork::{BlockchainDb, BlockchainDbMeta, SharedBackend},
    revm,
    revm::{BlockEnv, CfgEnv, TxEnv},
};
use parking_lot::{Mutex, RwLock};
use std::sync::Arc;

pub async fn spawn_fork(rpc_url: &str) -> Result<EthApi> {
    let config = NodeConfig {
        eth_rpc_url: Some(rpc_url.to_string()),
        enable_tracing: false,
        ..Default::default()
    };

    let backend = Arc::new(setup_node(config.clone()).await?);

    let NodeConfig {
        signer_accounts,
        transaction_order,
        ..
    } = config.clone();

    let pool = Arc::new(Pool::default());
    let miner = Miner::new(MiningMode::None);

    let dev_signer: Box<dyn EthSigner> = Box::new(DevSigner::new(signer_accounts));
    let fee_history_cache = Arc::new(Mutex::new(Default::default()));

    let filters = Filters::default();

    // create the cloneable api wrapper
    Ok(EthApi::new(
        Arc::clone(&pool),
        Arc::clone(&backend),
        Arc::new(vec![dev_signer]),
        fee_history_cache,
        1,
        miner.clone(),
        Default::default(),
        filters.clone(),
        transaction_order,
    ))
}

/// Configures everything related to env, backend and database and returns the
/// [Backend](mem::Backend)
///
/// *Note*: only memory based backend for now
async fn setup_node(mut config: NodeConfig) -> Result<mem::Backend> {
    // configure the revm environment
    let mut env = revm::Env {
        cfg: CfgEnv {
            spec_id: config.get_hardfork().into(),
            chain_id: config.get_chain_id().into(),
            ..Default::default()
        },
        block: BlockEnv {
            gas_limit: config.gas_limit,
            basefee: config.get_base_fee(),
            ..Default::default()
        },
        tx: TxEnv {
            chain_id: config.get_chain_id().into(),
            ..Default::default()
        },
    };
    let fees = FeeManager::new(
        env.cfg.spec_id,
        config.get_base_fee(),
        config.get_gas_price(),
    );
    let eth_rpc_url = config
        .eth_rpc_url
        .clone()
        .ok_or(anyhow!("eth_rpc_url is required"))?;
    let provider = Arc::new(Provider::<RetryClient<Http>>::new_client(
        &eth_rpc_url,
        10,
        1000,
    )?);

    // pick the last block number but also ensure it's not pending anymore
    let fork_block_number = find_latest_fork_block(&provider).await?;

    let block = provider
        .get_block(BlockNumber::Number(fork_block_number.into()))
        .await?
        .ok_or(anyhow!("block not found"))?;

    env.block.number = fork_block_number.into();
    let fork_timestamp = Some(block.timestamp);

    if let Some(base_fee) = block.base_fee_per_gas {
        config.base_fee = Some(base_fee);
        env.block.basefee = base_fee;
        // this is the base fee of the current block, but we need the base fee of the
        // next block
        let next_block_base_fee = fees.get_next_block_base_fee_per_gas(
            block.gas_used,
            block.gas_limit,
            block.base_fee_per_gas.unwrap_or_default(),
        );
        // update next base fee
        fees.set_base_fee(next_block_base_fee.into());
    }

    if let Ok(gas_price) = provider.get_gas_price().await {
        config.gas_price = Some(gas_price);
        fees.set_gas_price(gas_price);
    }

    let block_hash = block.hash.ok_or(anyhow!("No block hash"))?;

    let chain_id = provider.get_chainid().await?.as_u64();
    // need to update the dev signers and env with the chain id
    config.set_chain_id(Some(chain_id));
    env.cfg.chain_id = chain_id.into();
    env.tx.chain_id = chain_id.into();

    let override_chain_id = config.chain_id;

    let meta = BlockchainDbMeta::new(env.clone(), eth_rpc_url.clone());

    let block_chain_db = BlockchainDb::new(meta, config.block_cache_path());

    // This will spawn the background thread that will use the provider to fetch
    // blockchain data from the other client
    let backend = SharedBackend::spawn_backend_thread(
        Arc::clone(&provider),
        block_chain_db.clone(),
        Some(fork_block_number.into()),
    );

    let db = Arc::new(RwLock::new(ForkedDatabase::new(backend, block_chain_db)));
    let fork = ClientFork::new(
        ClientForkConfig {
            eth_rpc_url,
            block_number: fork_block_number,
            block_hash,
            provider,
            chain_id,
            override_chain_id,
            timestamp: block.timestamp.as_u64(),
            base_fee: block.base_fee_per_gas,
        },
        Arc::clone(&db),
    );

    let (db, fork): (Arc<RwLock<dyn Db>>, Option<ClientFork>) = (db, Some(fork));

    let genesis = GenesisConfig {
        balance: config.genesis_balance,
        accounts: config
            .genesis_accounts
            .iter()
            .map(|acc| acc.address())
            .collect(),
    };

    let backend = mem::Backend::with_genesis(db, Arc::new(RwLock::new(env)), genesis, fees, fork);

    if let Some(timestamp) = fork_timestamp {
        backend.time().set_start_timestamp(timestamp.as_u64());
    }
    Ok(backend)
}

/// Finds the latest appropriate block to fork
///
/// This fetches the "latest" block and checks whether the `Block` is fully populated (`hash` field
/// is present). This prevents edge cases where anvil forks the "latest" block but `eth_getBlockByNumber` still returns a pending block, <https://github.com/foundry-rs/foundry/issues/2036>
async fn find_latest_fork_block<M: Middleware>(provider: M) -> Result<u64, M::Error> {
    let mut num = provider.get_block_number().await?.as_u64();

    // walk back from the head of the chain, but at most 2 blocks, which should be more than enough
    // leeway
    for _ in 0..2 {
        if let Some(block) = provider.get_block(num).await? {
            if block.hash.is_some() {
                break;
            }
        }
        // block not actually finalized, so we try the block before
        num = num.saturating_sub(1)
    }

    Ok(num)
}
