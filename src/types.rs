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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    PolygonZkEVM,
    Gnosis,
    Celo,
    Moonbeam,
    Aurora,
}

impl Chain {
    pub fn chain_id(&self) -> u64 {
        match self {
            Chain::Ethereum => 1,
            Chain::BinanceSmartChain => 56,
            Chain::Polygon => 137,
            Chain::Arbitrum => 42161,
            Chain::Optimism => 10,
            Chain::Avalanche => 43114,
            Chain::Fantom => 250,
            Chain::Solana => 0,
            Chain::Base => 8453,
            Chain::ZkSync => 324,
            Chain::Linea => 59144,
            Chain::Scroll => 534352,
            Chain::Blast => 81457,
            Chain::PolygonZkEVM => 1101,
            Chain::Gnosis => 100,
            Chain::Celo => 42220,
            Chain::Moonbeam => 1284,
            Chain::Aurora => 1313161554,
        }
    }

    pub fn all_production_chains() -> Vec<Chain> {
        vec![
            Chain::Ethereum,
            Chain::BinanceSmartChain,
            Chain::Polygon,
            Chain::Arbitrum,
            Chain::Optimism,
            Chain::Avalanche,
            Chain::Fantom,
            Chain::Base,
            Chain::ZkSync,
            Chain::Linea,
            Chain::Scroll,
            Chain::Blast,
            Chain::PolygonZkEVM,
            Chain::Gnosis,
            Chain::Celo,
            Chain::Moonbeam,
            Chain::Aurora,
        ]
    }
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

// Additional types for exchanges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub chain_id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub base: Token,
    pub quote: Token,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub bid: Decimal,
    pub ask: Decimal,
    pub bid_size: Decimal,
    pub ask_size: Decimal,
    pub timestamp: DateTime<Utc>,
    pub exchange: String,
    pub pair: TokenPair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub price: Decimal,
    pub quantity: Decimal,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBook {
    pub exchange: String,
    pub pair: TokenPair,
    pub bids: Vec<Order>,
    pub asks: Vec<Order>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeFees {
    pub maker_fee: Decimal,
    pub taker_fee: Decimal,
    pub withdrawal_fee: std::collections::HashMap<String, Decimal>,
}

// Additional types for arbitrage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanInfo {
    pub provider: String,
    pub fee: Decimal,
    pub fee_percentage: Decimal,
    pub max_amount: Decimal,
}

// ML types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageDetector;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollector;

impl Default for SharedState {
    fn default() -> Self {
        Self {
            liquidity_pools: Arc::new(DashMap::new()),
            gas_prices: Arc::new(DashMap::new()),
            opportunities: Arc::new(DashMap::new()),
            historical_data: Arc::new(DashMap::new()),
        }
    }
}