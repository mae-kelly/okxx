use ethers::prelude::*;
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    pub ws_url: String,
    pub http_url: String,
    pub min_profit_wei: U256,
    pub max_gas_price: U256,
    pub monitoring_pairs: Vec<(Address, Address)>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            ws_url: std::env::var("WS_URL").unwrap_or_else(|_| 
                "wss://arb-mainnet.g.alchemy.com/v2/YOUR_KEY".to_string()),
            http_url: std::env::var("RPC_URL").unwrap_or_else(|_|
                "https://arb1.arbitrum.io/rpc".to_string()),
            min_profit_wei: U256::from(10u64.pow(16)), // 0.01 ETH minimum profit
            max_gas_price: U256::from(10u64.pow(9) * 100), // 100 Gwei max
            monitoring_pairs: vec![],
        })
    }
}
