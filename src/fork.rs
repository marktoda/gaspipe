use ethers::types::U256;
use forge::executor::{
    fork::CreateFork, inspector::CheatsConfig, opts::EvmOpts, Backend, Executor, ExecutorBuilder,
};
use foundry_config::Config;
use foundry_evm::revm::SpecId;

pub async fn spawn_fork(rpc_url: &str) -> Executor {
    let evm_opts = EvmOpts {
        fork_url: Some(rpc_url.to_string()),
        memory_limit: 1024 * 256,
        ..Default::default()
    };
    let env = evm_opts.evm_env().await;
    let fork = Some(CreateFork {
        url: rpc_url.to_string(),
        enable_caching: true,
        env: env.clone(),
        evm_opts: evm_opts.clone(),
    });

    let config: Config = Default::default();

    // the db backend that serves all the data
    let db = Backend::spawn(fork);

    ExecutorBuilder::default()
        .with_cheatcodes(CheatsConfig::new(&config, &evm_opts))
        .with_config(env)
        .with_spec(SpecId::LONDON)
        .with_gas_limit(U256::from_dec_str("30000000").unwrap())
        .set_tracing(false)
        .set_debugger(false)
        .build(db)
}
