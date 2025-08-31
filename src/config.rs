use anyhow::Result;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub chains: HashMap<String, ChainConfig>,
    pub flash_loan_providers: Vec<FlashLoanConfig>,
    pub dexs: HashMap<String, DexConfig>,
    pub min_profit_usd: Decimal,
    pub max_gas_price_gwei: Decimal,
    pub scan_interval_ms: u64,
    pub websocket_endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub enabled: bool,
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanConfig {
    pub name: String,
    pub chain: String,
    pub contract_address: String,
    pub fee_percentage: Decimal,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    pub factory_address: String,
    pub router_address: String,
    pub enabled: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();
        
        let mut chains = HashMap::new();
        
        // Ethereum
        chains.insert("ethereum".to_string(), ChainConfig {
            enabled: true,
            rpc_url: std::env::var("ETH_RPC_URL")
                .unwrap_or_else(|_| "https://eth.llamarpc.com".to_string()),
            ws_url: std::env::var("ETH_WS_URL").ok(),
            chain_id: 1,
        });
        
        // BSC
        chains.insert("bsc".to_string(), ChainConfig {
            enabled: true,
            rpc_url: std::env::var("BSC_RPC_URL")
                .unwrap_or_else(|_| "https://bsc-dataseed.binance.org".to_string()),
            ws_url: None,
            chain_id: 56,
        });
        
        // Polygon
        chains.insert("polygon".to_string(), ChainConfig {
            enabled: true,
            rpc_url: std::env::var("POLYGON_RPC_URL")
                .unwrap_or_else(|_| "https://polygon-rpc.com".to_string()),
            ws_url: None,
            chain_id: 137,
        });
        
        // Arbitrum
        chains.insert("arbitrum".to_string(), ChainConfig {
            enabled: true,
            rpc_url: std::env::var("ARBITRUM_RPC_URL")
                .unwrap_or_else(|_| "https://arb1.arbitrum.io/rpc".to_string()),
            ws_url: None,
            chain_id: 42161,
        });
        
        // Flash loan providers
        let flash_loan_providers = vec![
            FlashLoanConfig {
                name: "Aave V3".to_string(),
                chain: "ethereum".to_string(),
                contract_address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
                fee_percentage: Decimal::from_str_exact("0.0009").unwrap(),
                enabled: true,
            },
            FlashLoanConfig {
                name: "Balancer".to_string(),
                chain: "ethereum".to_string(),
                contract_address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
                fee_percentage: Decimal::ZERO,
                enabled: true,
            },
            FlashLoanConfig {
                name: "dYdX".to_string(),
                chain: "ethereum".to_string(),
                contract_address: "0x1E0447b19BB6EcFdAe1e4AE1694b0C3659614e4e".to_string(),
                fee_percentage: Decimal::ZERO,
                enabled: true,
            },
            FlashLoanConfig {
                name: "Uniswap V3".to_string(),
                chain: "ethereum".to_string(),
                contract_address: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45".to_string(),
                fee_percentage: Decimal::from_str_exact("0.0001").unwrap(),
                enabled: true,
            },
            FlashLoanConfig {
                name: "Aave V3".to_string(),
                chain: "polygon".to_string(),
                contract_address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
                fee_percentage: Decimal::from_str_exact("0.0009").unwrap(),
                enabled: true,
            },
            FlashLoanConfig {
                name: "Aave V3".to_string(),
                chain: "arbitrum".to_string(),
                contract_address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
                fee_percentage: Decimal::from_str_exact("0.0009").unwrap(),
                enabled: true,
            },
        ];
        
        // DEX configurations
        let mut dexs = HashMap::new();
        
        dexs.insert("uniswap_v2".to_string(), DexConfig {
            factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".to_string(),
            router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".to_string(),
            enabled: true,
        });
        
        dexs.insert("uniswap_v3".to_string(), DexConfig {
            factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string(),
            router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
            enabled: true,
        });
        
        dexs.insert("sushiswap".to_string(), DexConfig {
            factory_address: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".to_string(),
            router_address: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string(),
            enabled: true,
        });
        
        Ok(Config {
            chains,
            flash_loan_providers,
            dexs,
            min_profit_usd: Decimal::from(50),
            max_gas_price_gwei: Decimal::from(100),
            scan_interval_ms: 500,
            websocket_endpoints: vec![
                "wss://stream.binance.com:9443/ws".to_string(),
                "wss://ws-feed.exchange.coinbase.com".to_string(),
            ],
        })
    }
}

use rust_decimal::prelude::FromStr;