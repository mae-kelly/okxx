use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rust_decimal::Decimal;
use rust_decimal::prelude::FromStr;

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
    pub notifications: NotificationConfig,
    pub wallet: WalletConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangesConfig {
    pub okx: OkxCredentials,
    pub uniswap_v3: DexConfig,
    pub sushiswap: DexConfig,
    pub pancakeswap: DexConfig,
    pub curve: DexConfig,
    pub balancer: DexConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OkxCredentials {
    pub api_key: String,
    pub api_secret: String,
    pub passphrase: String,
    pub enabled: bool,
    pub ws_public: String,
    pub ws_private: String,
    pub rest_url: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    pub discord_webhook: Option<String>,
    pub discord_min_profit: f64,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    pub email_smtp: Option<String>,
    pub email_from: Option<String>,
    pub email_to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub monitor_address: String,
    pub executor_address: String,
    pub executor_private_key: String,
    pub max_position_size_usd: Decimal,
    pub max_daily_trades: u32,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();
        
        // Load API keys from environment
        let okx_api_key = std::env::var("OKX_API_KEY")
            .unwrap_or_else(|_| "8a760df1-4a2d-471b-ba42-d16893614dab".to_string());
        let okx_secret = std::env::var("OKX_SECRET_KEY")
            .unwrap_or_else(|_| "C9F3FC89A6A30226E11DFFD098C7CF3D".to_string());
        let okx_passphrase = std::env::var("OKX_PASSPHRASE").unwrap_or_default();
        
        let alchemy_key = std::env::var("ALCHEMY_API_KEY")
            .unwrap_or_else(|_| "alcht_oZ7wU7JpIoZejlOWUcMFOpNsIlLDsX".to_string());
        let infura_key = std::env::var("INFURA_API_KEY")
            .unwrap_or_else(|_| "2e1c7909e5e4488e99010fabd3590a79".to_string());
        
        let etherscan_key = std::env::var("ETHERSCAN_API_KEY")
            .unwrap_or_else(|_| "K4SEVFZ3PI8STM73VKV84C8PYZJUK7HB2G".to_string());
        
        let discord_webhook = std::env::var("DISCORD_WEBHOOK_URL")
            .unwrap_or_else(|_| "https://discord.com/api/webhooks/1398448251933298740/lSnT3iPsfvb87RWdN0XCd3AjdFsCZiTpF-_I1ciV3rB2BqTpIszS6U6tFxAVk5QmM2q3".to_string());
        
        let config = Self {
            exchanges: ExchangesConfig {
                okx: OkxCredentials {
                    api_key: okx_api_key,
                    api_secret: okx_secret,
                    passphrase: okx_passphrase,
                    enabled: true,
                    ws_public: "wss://ws.okx.com:8443/ws/v5/public".to_string(),
                    ws_private: "wss://ws.okx.com:8443/ws/v5/private".to_string(),
                    rest_url: "https://www.okx.com".to_string(),
                },
                uniswap_v3: DexConfig {
                    router_address: vec![
                        (1, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        (137, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        (42161, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        (10, "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        (8453, "0x2626664c2603336E57B271c5C0b26F421741e481".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (1, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                        (137, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                        (42161, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                        (10, "0x1F98431c8aD98523631AE4a59f267346ea31F984".to_string()),
                        (8453, "0x33128a8fC17869897dcE68Ed026d694621f6FDfD".to_string()),
                    ].into_iter().collect(),
                    enabled: true,
                },
                sushiswap: DexConfig {
                    router_address: vec![
                        (1, "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".to_string()),
                        (137, "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string()),
                        (42161, "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".to_string()),
                    ].into_iter().collect(),
                    factory_address: vec![
                        (1, "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".to_string()),
                        (137, "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".to_string()),
                        (42161, "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".to_string()),
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
                        format!("https://eth-mainnet.g.alchemy.com/v2/{}", alchemy_key),
                        format!("https://mainnet.infura.io/v3/{}", infura_key),
                    ],
                    ws_url: Some(format!("wss://eth-mainnet.g.alchemy.com/v2/{}", alchemy_key)),
                    native_token: "ETH".to_string(),
                    explorer_api_key: Some(etherscan_key.clone()),
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 137,
                    name: "Polygon".to_string(),
                    rpc_urls: vec![
                        format!("https://polygon-mainnet.g.alchemy.com/v2/{}", alchemy_key),
                        format!("https://polygon-mainnet.infura.io/v3/{}", infura_key),
                    ],
                    ws_url: Some(format!("wss://polygon-mainnet.g.alchemy.com/v2/{}", alchemy_key)),
                    native_token: "MATIC".to_string(),
                    explorer_api_key: None,
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 42161,
                    name: "Arbitrum".to_string(),
                    rpc_urls: vec![
                        format!("https://arb-mainnet.g.alchemy.com/v2/{}", alchemy_key),
                        format!("https://arbitrum-mainnet.infura.io/v3/{}", infura_key),
                    ],
                    ws_url: Some(format!("wss://arb-mainnet.g.alchemy.com/v2/{}", alchemy_key)),
                    native_token: "ETH".to_string(),
                    explorer_api_key: None,
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 10,
                    name: "Optimism".to_string(),
                    rpc_urls: vec![
                        format!("https://opt-mainnet.g.alchemy.com/v2/{}", alchemy_key),
                        format!("https://optimism-mainnet.infura.io/v3/{}", infura_key),
                    ],
                    ws_url: Some(format!("wss://opt-mainnet.g.alchemy.com/v2/{}", alchemy_key)),
                    native_token: "ETH".to_string(),
                    explorer_api_key: None,
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 8453,
                    name: "Base".to_string(),
                    rpc_urls: vec![
                        format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key),
                    ],
                    ws_url: Some(format!("wss://base-mainnet.g.alchemy.com/v2/{}", alchemy_key)),
                    native_token: "ETH".to_string(),
                    explorer_api_key: None,
                    enabled: true,
                },
                ChainConfig {
                    chain_id: 56,
                    name: "BSC".to_string(),
                    rpc_urls: vec![
                        "https://bsc-dataseed1.binance.org".to_string(),
                        "https://bsc-dataseed2.binance.org".to_string(),
                    ],
                    ws_url: None,
                    native_token: "BNB".to_string(),
                    explorer_api_key: None,
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
            ],
            scanner: ScannerConfig {
                scan_interval_ms: std::env::var("SCAN_INTERVAL_MS")
                    .unwrap_or_else(|_| "500".to_string())
                    .parse().unwrap_or(500),
                concurrent_scans: 10,
                orderbook_depth: 20,
                min_liquidity_usd: Decimal::from(1000),
                max_slippage_percentage: Decimal::from_str_exact("0.005").unwrap(),
                price_update_threshold_percentage: Decimal::from_str_exact("0.001").unwrap(),
            },
            database: DatabaseConfig {
                mongodb_uri: std::env::var("MONGODB_URI")
                    .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
                redis_uri: std::env::var("REDIS_URI")
                    .unwrap_or_else(|_| "redis://localhost:6379".to_string()),
                database_name: "arbitrage_scanner_prod".to_string(),
                retention_days: 30,
            },
            ml: MLConfig {
                model_path: "./models".to_string(),
                retrain_interval_hours: 6,
                min_training_samples: 100,
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
                min_profit_usd: Decimal::from(25),
                max_gas_percentage: Decimal::from_str_exact("0.2").unwrap(),
                min_confidence_score: 0.7,
            },
            notifications: NotificationConfig {
                discord_webhook: Some(discord_webhook),
                discord_min_profit: std::env::var("DISCORD_ALERT_MIN_PROFIT")
                    .unwrap_or_else(|_| "50".to_string())
                    .parse().unwrap_or(50.0),
                telegram_bot_token: None,
                telegram_chat_id: None,
                email_smtp: None,
                email_from: None,
                email_to: vec![],
            },
            wallet: WalletConfig {
                monitor_address: std::env::var("MONITOR_WALLET_ADDRESS")
                    .unwrap_or_else(|_| "0xB06bB023c084A34f410F1069EbD467bEA83ADaB2".to_string()),
                executor_address: std::env::var("EXECUTOR_WALLET_ADDRESS")
                    .unwrap_or_else(|_| "0x0Ca2D41fD5062D90a20c45259daA280910ed4C7c".to_string()),
                executor_private_key: std::env::var("EXECUTOR_PRIVATE_KEY")
                    .unwrap_or_else(|_| "0x2cded561032136fb4aecb8b89b7d7e4a54b86d2d0b98f5f3b635de4a44984c37".to_string()),
                max_position_size_usd: Decimal::from(10000),
                max_daily_trades: 100,
            },
        };
        
        Ok(config)
    }
}