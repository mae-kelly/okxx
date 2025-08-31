use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Chain definitions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Chain {
    Ethereum,
    BinanceSmartChain,
    Polygon,
    Arbitrum,
    Optimism,
    Avalanche,
    Base,
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
            Chain::Base => 8453,
        }
    }
    
    pub fn name(&self) -> &str {
        match self {
            Chain::Ethereum => "Ethereum",
            Chain::BinanceSmartChain => "BSC",
            Chain::Polygon => "Polygon",
            Chain::Arbitrum => "Arbitrum",
            Chain::Optimism => "Optimism",
            Chain::Avalanche => "Avalanche",
            Chain::Base => "Base",
        }
    }
    
    pub fn all() -> Vec<Chain> {
        vec![
            Chain::Ethereum,
            Chain::BinanceSmartChain,
            Chain::Polygon,
            Chain::Arbitrum,
            Chain::Optimism,
            Chain::Avalanche,
            Chain::Base,
        ]
    }
    
    pub fn native_token(&self) -> &str {
        match self {
            Chain::Ethereum => "ETH",
            Chain::BinanceSmartChain => "BNB",
            Chain::Polygon => "MATIC",
            Chain::Arbitrum => "ETH",
            Chain::Optimism => "ETH",
            Chain::Avalanche => "AVAX",
            Chain::Base => "ETH",
        }
    }
}

// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: String,
    pub symbol: String,
    pub decimals: u8,
    pub chain: Chain,
}

// Liquidity pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityPool {
    pub address: String,
    pub token0: Token,
    pub token1: Token,
    pub reserve0: Decimal,
    pub reserve1: Decimal,
    pub fee: Decimal,
    pub dex: String,
    pub chain: Chain,
    pub last_update: DateTime<Utc>,
}

// Price data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceData {
    pub token_pair: String,
    pub price: Decimal,
    pub liquidity: Decimal,
    pub volume_24h: Decimal,
    pub source: String,
    pub chain: Chain,
    pub timestamp: DateTime<Utc>,
}

// Gas price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasPrice {
    pub chain: Chain,
    pub fast: Decimal,
    pub standard: Decimal,
    pub slow: Decimal,
    pub timestamp: DateTime<Utc>,
}

// Flash loan provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashLoanProvider {
    pub name: String,
    pub chain: Chain,
    pub contract_address: String,
    pub fee_percentage: Decimal,
    pub available_tokens: Vec<String>,
}

// Arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    pub id: String,
    pub chain: Chain,
    pub opportunity_type: String,
    pub path: Vec<TradePath>,
    pub initial_amount: Decimal,
    pub final_amount: Decimal,
    pub gross_profit: Decimal,
    pub flash_loan_provider: String,
    pub flash_loan_fee: Decimal,
    pub flash_loan_fee_percentage: Decimal,
    pub gas_cost_usd: Decimal,
    pub net_profit_usd: Decimal,
    pub roi_percentage: Decimal,
    pub confidence_score: f64,
    pub timestamp: DateTime<Utc>,
}

// Trade path component
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradePath {
    pub dex: String,
    pub pool_address: String,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: Decimal,
    pub amount_out: Decimal,
}

// Shared application state
pub struct SharedState {
    pub prices: Arc<DashMap<String, PriceData>>,
    pub pools: Arc<DashMap<String, LiquidityPool>>,
    pub gas_prices: Arc<DashMap<Chain, GasPrice>>,
    pub opportunities: Arc<RwLock<Vec<ArbitrageOpportunity>>>,
}