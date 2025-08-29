#![allow(dead_code)]
use dashmap::DashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub token_pair: String,
    pub price: Decimal,
    pub volume_24h: Decimal,
    pub liquidity: Decimal,
    pub exchange: String,
    pub chain: Chain,
    pub timestamp: DateTime<Utc>,
    pub bid: Decimal,
    pub ask: Decimal,
    pub spread: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Chain {
    Ethereum,
    BinanceSmartChain,
    Polygon,
    Arbitrum,
    Optimism,
    Avalanche,
    Fantom,
    Solana,
    Base,
    ZkSync,
    Linea,
    Scroll,
    Blast,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    pub address: String,
    pub token0: TokenInfo,
    pub token1: TokenInfo,
    pub reserve0: Decimal,
    pub reserve1: Decimal,
    pub fee: Decimal,
    pub exchange: String,
    pub chain: Chain,
    pub pool_type: PoolType,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolType {
    UniswapV2,
    UniswapV3,
    Curve,
    Balancer,
    PancakeV2,
    PancakeV3,
    SushiSwap,
    QuickSwap,
    TraderJoe,
    SpookySwap,
    Raydium,
    Orca,
    Camelot,
    Velodrome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub price_usd: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub path: Vec<TradeLeg>,
    pub initial_amount: Decimal,
    pub final_amount: Decimal,
    pub profit_amount: Decimal,
    pub profit_usd: f64,
    pub roi_percentage: f64,
    pub total_gas_cost: Decimal,
    pub flash_loan_fee: Decimal,
    pub chain: Chain,
    pub timestamp: DateTime<Utc>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLeg {
    pub exchange: String,
    pub pool_address: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
    pub price: Decimal,
    pub fee: Decimal,
    pub gas_estimate: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPrice {
    pub chain: Chain,
    pub fast: Decimal,
    pub standard: Decimal,
    pub slow: Decimal,
    pub base_fee: Decimal,
    pub priority_fee: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanProvider {
    pub name: String,
    pub chain: Chain,
    pub fee_percentage: Decimal,
    pub max_loan_amount: std::collections::HashMap<String, Decimal>,
    pub contract_address: String,
}

#[derive(Debug, Clone)]
pub struct SharedState {
    pub prices: Arc<DashMap<String, PriceData>>,
    pub liquidity_pools: Arc<DashMap<String, LiquidityPool>>,
    pub gas_prices: Arc<DashMap<Chain, GasPrice>>,
    pub opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
    pub historical_data: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MLInsights {
    pub most_profitable_chains: Vec<(Chain, f64)>,
    pub most_profitable_exchanges: Vec<(String, f64)>,
    pub most_profitable_tokens: Vec<(String, f64)>,
    pub best_time_windows: Vec<TimeWindow>,
    pub average_profit_by_chain: Vec<(Chain, f64)>,
    pub opportunity_frequency: Vec<(String, u64)>,
    pub prediction_accuracy: f64,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeWindow {
    pub hour: u8,
    pub day_of_week: u8,
    pub avg_opportunities: f64,
    pub avg_profit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketMessage {
    pub msg_type: MessageType,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Price,
    OrderBook,
    Trade,
    Liquidity,
    Block,
    Mempool,
}