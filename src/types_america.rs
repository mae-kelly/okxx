use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use parking_lot;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageSignal {
    pub id: String,
    pub buy_exchange: String,
    pub sell_exchange: String,
    pub token_pair: String,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub volume: Decimal,
    pub profit: Decimal,
    pub roi: Decimal,
    pub gas_cost: Decimal,
    pub flash_loan_fee: Decimal,
    pub total_fees: Decimal,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketFeed {
    pub name: String,
    pub url: String,
    pub subscription: serde_json::Value,
    pub chain: Option<String>,
    pub feed_type: FeedType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedType {
    CEX,
    DEX,
    Aggregator,
    Oracle,
    Analytics,
}

impl WebSocketFeed {
    pub fn new(name: &str, url: &str, subscription: serde_json::Value) -> Self {
        let feed_type = if name.contains("Binance") || name.contains("Coinbase") || name.contains("Kraken") {
            FeedType::CEX
        } else if name.contains("Uniswap") || name.contains("Sushi") || name.contains("Pancake") {
            FeedType::DEX
        } else if name.contains("1inch") || name.contains("0x") || name.contains("Paraswap") {
            FeedType::Aggregator
        } else if name.contains("Chainlink") || name.contains("Band") || name.contains("Pyth") {
            FeedType::Oracle
        } else {
            FeedType::Analytics
        };
        
        Self {
            name: name.to_string(),
            url: url.to_string(),
            subscription,
            chain: None,
            feed_type,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SharedState {
    pub signals: Arc<DashMap<String, ArbitrageSignal>>,
    pub price_index: Arc<parking_lot::RwLock<HashMap<String, Decimal>>>,
    pub opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
    pub gas_tracker: Arc<GasTracker>,
    pub flash_loan_optimizer: Arc<FlashLoanOptimizer>,
    pub performance_stats: Arc<DashMap<String, WebSocketStats>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub strategy: Strategy,
    pub exchanges: Vec<String>,
    pub token_path: Vec<String>,
    pub expected_profit: Decimal,
    pub required_capital: Decimal,
    pub gas_estimate: Decimal,
    pub flash_loan_provider: String,
    pub flash_loan_fee: Decimal,
    pub total_fees: Decimal,
    pub net_profit: Decimal,
    pub roi_percentage: f64,
    pub confidence_score: f64,
    pub risk_level: RiskLevel,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Strategy {
    Triangular,
    CrossExchange,
    FlashLoanArbitrage,
    Sandwich,
    Liquidation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone)]
pub struct GasTracker {
    pub ethereum_gas: Arc<RwLock<GasPrice>>,
    pub bsc_gas: Arc<RwLock<GasPrice>>,
    pub polygon_gas: Arc<RwLock<GasPrice>>,
    pub arbitrum_gas: Arc<RwLock<GasPrice>>,
    pub optimism_gas: Arc<RwLock<GasPrice>>,
}

impl GasTracker {
    pub fn new() -> Self {
        Self {
            ethereum_gas: Arc::new(RwLock::new(GasPrice::default())),
            bsc_gas: Arc::new(RwLock::new(GasPrice::default())),
            polygon_gas: Arc::new(RwLock::new(GasPrice::default())),
            arbitrum_gas: Arc::new(RwLock::new(GasPrice::default())),
            optimism_gas: Arc::new(RwLock::new(GasPrice::default())),
        }
    }
    
    pub async fn get_current_gas_price(&self) -> Decimal {
        // Return Ethereum gas as default
        self.ethereum_gas.read().await.fast
    }
    
    pub async fn start_monitoring(&self) {
        // Monitoring implementation would go here
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
            // Update gas prices from various sources
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct GasPrice {
    pub slow: Decimal,
    pub standard: Decimal,
    pub fast: Decimal,
    pub instant: Decimal,
    pub base_fee: Decimal,
    pub priority_fee: Decimal,
    pub max_fee: Decimal,
    pub timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct FlashLoanOptimizer {
    pub providers: Vec<FlashLoanProvider>,
}

impl FlashLoanOptimizer {
    pub fn new() -> Self {
        Self {
            providers: vec![
                FlashLoanProvider {
                    name: "Aave V3".to_string(),
                    chains: vec!["Ethereum", "Polygon", "Arbitrum", "Optimism", "Avalanche"].into_iter().map(String::from).collect(),
                    fee_percentage: Decimal::from_str_exact("0.0009").unwrap(),
                    max_loan: Decimal::from(100_000_000),
                },
                FlashLoanProvider {
                    name: "Balancer".to_string(),
                    chains: vec!["Ethereum", "Polygon", "Arbitrum"].into_iter().map(String::from).collect(),
                    fee_percentage: Decimal::ZERO,
                    max_loan: Decimal::from(50_000_000),
                },
                FlashLoanProvider {
                    name: "dYdX".to_string(),
                    chains: vec!["Ethereum"].into_iter().map(String::from).collect(),
                    fee_percentage: Decimal::from_str_exact("0.0002").unwrap(),
                    max_loan: Decimal::from(10_000_000),
                },
                FlashLoanProvider {
                    name: "Uniswap V3".to_string(),
                    chains: vec!["Ethereum", "Polygon", "Arbitrum", "Optimism"].into_iter().map(String::from).collect(),
                    fee_percentage: Decimal::from_str_exact("0.0001").unwrap(),
                    max_loan: Decimal::from(200_000_000),
                },
                FlashLoanProvider {
                    name: "PancakeSwap".to_string(),
                    chains: vec!["BSC"].into_iter().map(String::from).collect(),
                    fee_percentage: Decimal::from_str_exact("0.0025").unwrap(),
                    max_loan: Decimal::from(20_000_000),
                },
            ],
        }
    }
    
    pub fn get_best_provider(&self, chain: &str, amount: Decimal) -> Option<&FlashLoanProvider> {
        self.providers
            .iter()
            .filter(|p| p.chains.contains(&chain.to_string()) && p.max_loan >= amount)
            .min_by_key(|p| (p.fee_percentage * Decimal::from(10000)).to_u64().unwrap_or(u64::MAX))
    }
}

#[derive(Debug, Clone)]
pub struct FlashLoanProvider {
    pub name: String,
    pub chains: Vec<String>,
    pub fee_percentage: Decimal,
    pub max_loan: Decimal,
}

#[derive(Debug, Clone, Default)]
pub struct WebSocketStats {
    pub messages_received: u64,
    pub opportunities_found: u64,
    pub total_profit: Decimal,
    pub avg_latency_ms: f64,
    pub uptime_seconds: u64,
    pub last_message: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLPrediction {
    pub confidence: f32,
    pub quality_score: f32,
    pub recommended_action: String,
    pub risk_level: String,
    pub expected_profit: f32,
    pub websocket_ranking: u32,
}

use rust_decimal::prelude::*;