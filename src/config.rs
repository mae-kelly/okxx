use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub exchanges: ExchangesConfig,
    pub chains: Vec<ChainConfig>,
    pub flash_loan_providers: Vec<FlashLoanConfig>,
    pub scanner: ScannerConfig,
    pub database: DatabaseConfig,
    pub ml: MLConfig,
    pub websocket: WebSocketConfig,
    pub thresholds: ThresholdConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangesConfig {
    pub binance: ExchangeCredentials,
    pub coinbase: ExchangeCredentials,
    pub kraken: ExchangeCredentials,
    pub uniswap_v3: DexConfig,
    pub sushiswap: DexConfig,
    pub pancakeswap: DexConfig,
    pub curve: DexConfig,
    pub balancer: DexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeCredentials {
    pub api_key: String,
    pub api_secret: String,
    pub enabled: bool,
    pub rate_limit_per_second: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    pub router_address: HashMap<u64, String>,
    pub factory_address: HashMap<u64, String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: String,
    pub rpc_urls: Vec<String>,
    pub ws_url: Option<String>,
    pub native_token: String,
    pub explorer_api_key: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanConfig {
    pub provider: String,
    pub chain_id: u64,
    pub pool_address: String,
    pub fee_percentage: Decimal,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannerConfig {
    pub scan_interval_ms: u64,
    pub concurrent_scans: usize,
    pub orderbook_depth: usize,
    pub min_liquidity_usd: Decimal,
    pub max_slippage_percentage: Decimal,
    pub price_update_threshold_percentage: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub mongodb_uri: String,
    pub redis_uri: String,
    pub database_name: String,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLConfig {
    pub model_path: String,
    pub retrain_interval_hours: u32,
    pub min_training_samples: usize,
    pub feature_importance_threshold: f64,
    pub cross_validation_folds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    pub port: u16,
    pub max_connections: usize,
    pub heartbeat_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub min_profit_percentage: Decimal,
    pub min_profit_usd: Decimal,
    pub max_gas_percentage: Decimal,
    pub min_confidence_score: f64,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();
        
        let config = Self {
            exchanges: ExchangesConfig {
                binance: ExchangeCredentials {
                    api_key: std::env::var("BINANCE_API_KEY").unwrap_or_default(),
                    api_secret: std::env::var("BINANCE_API_SECRET").unwrap_or_default(),
                    enabled: true,
                    rate_limit_per_second: 10,
                },
                coinbase: ExchangeCredentials {
                    api_key: std::env::var("COINBASE_API_KEY").unwrap_or_default(),
                    api_secret: std::env::var("COINBASE_API_SECRET").unwrap_or_default(),
                    enabled: true,
                    rate_limit_per_second: 10,
                },
                kraken: ExchangeCredentials {
                    api_key: std::env::var("KRAKEN_API_KEY").unwrap_or_default(),
                    api_secret: std::env::var("KRAKEN_API_SECRET").unwrap_or_default(),
                    enabled: true,
                    rate_limit_per_second: 6,
                },
                uniswap_v3: DexConfig {
                    router_address: vec![
                        (1, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        (137, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        (42161, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (1, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                        (137, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                        (42161, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                    ].into_iter().collect(),
                    enabled: true,
                },
                sushiswap: DexConfig {
                    router_address: vec![
                        (1, "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string()),
                        (137, "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (1, "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".to_string()),
                        (137, "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".to_string()),
                    ].into_iter().collect(),
                    enabled: true,
                },
                pancakeswap: DexConfig {
                    router_address: vec![
                        (56, "0x10ED43C718714eb63d5aA57B78B54704E256024E".to_string()),
                        (1, "0xEfF92A263d31888d860bD50809A8D171709b7b1c".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (56, "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73".to_string()),
                        (1, "0x1097053Fd2ea711dad45caCcc45EfF7548fCB362".to_string()),
                    ].into_iter().collect(),
                    enabled: true,
                },
                curve: DexConfig {
                    router_address: vec![
                        (1, "0x99a58482BD75cbab83b27EC03CA68fF489b5788f".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (1, "0xB9fC157394Af804a3578134A6585C0dc9cc990d4".to_string()),
                    ].into_iter().collect(),
                    enabled: true,
                },
                balancer: DexConfig {
                    router_address: vec![
                        (1, "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (1, "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string()),
                    ].into_iter().collect(),
                    enabled: true,
                },
            },
            chains: vec![
                ChainConfig {
                    chain_id: 1,
                    name: "Ethereum".to_string(),
                    rpc_urls: vec![
                        std::env::var("ETH_RPC_URL").unwrap_or_else(|_| "https://eth.llamarpc.com".to_string()),
                    ],
                    ws_url: std::env::var("ETH_WS_URL").ok(),
                    native_token: "ETH".to_string(),
                    explorer_api_key: std::env::var("ETHERSCAN_API_KEY").ok(),
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 56,
                    name: "BSC".to_string(),
                    rpc_urls: vec![
                        std::env::var("BSC_RPC_URL").unwrap_or_else(|_| "https://bsc-dataseed.binance.org".to_string()),
                    ],
                    ws_url: None,
                    native_token: "BNB".to_string(),
                    explorer_api_key: std::env::var("BSCSCAN_API_KEY").ok(),
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 137,
                    name: "Polygon".to_string(),
                    rpc_urls: vec![
                        std::env::var("POLYGON_RPC_URL").unwrap_or_else(|_| "https://polygon-rpc.com".to_string()),
                    ],
                    ws_url: None,
                    native_token: "MATIC".to_string(),
                    explorer_api_key: std::env::var("POLYGONSCAN_API_KEY").ok(),
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 42161,
                    name: "Arbitrum".to_string(),
                    rpc_urls: vec![
                        std::env::var("ARBITRUM_RPC_URL").unwrap_or_else(|_| "https://arb1.arbitrum.io/rpc".to_string()),
                    ],
                    ws_url: None,
                    native_token: "ETH".to_string(),
                    explorer_api_key: std::env::var("ARBISCAN_API_KEY").ok(),
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 10,
                    name: "Optimism".to_string(),
                    rpc_urls: vec![
                        std::env::var("OPTIMISM_RPC_URL").unwrap_or_else(|_| "https://mainnet.optimism.io".to_string()),
                    ],
                    ws_url: None,
                    native_token: "ETH".to_string(),
                    explorer_api_key: std::env::var("OPTIMISTIC_ETHERSCAN_API_KEY").ok(),
                    enabled: true,
                },
            ],
            flash_loan_providers: vec![
                FlashLoanConfig {
                    provider: "Aave V3".to_string(),
                    chain_id: 1,
                    pool_address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
                    fee_percentage: Decimal::from_str_exact("0.0009").unwrap(),
                    enabled: true,
                },
                FlashLoanConfig {
                    provider: "Balancer".to_string(),
                    chain_id: 1,
                    pool_address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
                    fee_percentage: Decimal::ZERO,
                    enabled: true,
                },
                FlashLoanConfig {
                    provider: "dYdX".to_string(),
                    chain_id: 1,
                    pool_address: "0x1E0447b19BB6EcFdAe1e4AE1694b0C3659614e4e".to_string(),
                    fee_percentage: Decimal::from_str_exact("0.0002").unwrap(),
                    enabled: true,
                },
            ],
            scanner: ScannerConfig {
                scan_interval_ms: 1000,
                concurrent_scans: 10,
                orderbook_depth: 20,
                min_liquidity_usd: Decimal::from(1000),
                max_slippage_percentage: Decimal::from_str_exact("0.005").unwrap(),
                price_update_threshold_percentage: Decimal::from_str_exact("0.001").unwrap(),
            },
            database: DatabaseConfig {
                mongodb_uri: std::env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
                redis_uri: std::env::var("REDIS_URI").unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                database_name: "arbitrage_scanner".to_string(),
                retention_days: 30,
            },
            ml: MLConfig {
                model_path: "./models".to_string(),
                retrain_interval_hours: 6,
                min_training_samples: 1000,
                feature_importance_threshold: 0.05,
                cross_validation_folds: 5,
            },
            websocket: WebSocketConfig {
                port: 8080,
                max_connections: 100,
                heartbeat_interval_secs: 30,
            },
            thresholds: ThresholdConfig {
                min_profit_percentage: Decimal::from_str_exact("0.005").unwrap(),
                min_profit_usd: Decimal::from(10),
                max_gas_percentage: Decimal::from_str_exact("0.3").unwrap(),
                min_confidence_score: 0.7,
            },
        };
        
        Ok(config)
    }
}

use rust_decimal::prelude::FromStr;