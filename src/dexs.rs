use std::sync::Arc;
use ethers::prelude::*;
use anyhow::Result;
use rust_decimal::Decimal;
use std::str::FromStr;
use crate::types::{Chain, LiquidityPool, TokenInfo, PoolType};
use crate::chains::ChainManager;
use chrono::Utc;
use tracing::{info, debug};

pub struct DexManager {
    chain_manager: Arc<ChainManager>,
    dex_configs: Vec<DexConfig>,
}

struct DexConfig {
    name: String,
    chain: Chain,
    factory_address: Address,
    router_address: Address,
    pool_type: PoolType,
    fee_percentage: Decimal,
}

impl DexManager {
    pub async fn new(chain_manager: Arc<ChainManager>) -> Result<Self> {
        let dex_configs = vec![
            DexConfig {
                name: "Uniswap V2".to_string(),
                chain: Chain::Ethereum,
                factory_address: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f".parse()?,
                router_address: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D".parse()?,
                pool_type: PoolType::UniswapV2,
                fee_percentage: Decimal::from_str("0.003")?,
            },
            DexConfig {
                name: "Uniswap V3".to_string(),
                chain: Chain::Ethereum,
                factory_address: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse()?,
                router_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse()?,
                pool_type: PoolType::UniswapV3,
                fee_percentage: Decimal::from_str("0.003")?,
            },
            DexConfig {
                name: "SushiSwap".to_string(),
                chain: Chain::Ethereum,
                factory_address: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac".parse()?,
                router_address: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F".parse()?,
                pool_type: PoolType::SushiSwap,
                fee_percentage: Decimal::from_str("0.003")?,
            },
            DexConfig {
                name: "PancakeSwap V2".to_string(),
                chain: Chain::BinanceSmartChain,
                factory_address: "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73".parse()?,
                router_address: "0x10ED43C718714eb63d5aA57B78B54704E256024E".parse()?,
                pool_type: PoolType::PancakeV2,
                fee_percentage: Decimal::from_str("0.0025")?,
            },
            DexConfig {
                name: "PancakeSwap V3".to_string(),
                chain: Chain::BinanceSmartChain,
                factory_address: "0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865".parse()?,
                router_address: "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4".parse()?,
                pool_type: PoolType::PancakeV3,
                fee_percentage: Decimal::from_str("0.0025")?,
            },
            DexConfig {
                name: "QuickSwap".to_string(),
                chain: Chain::Polygon,
                factory_address: "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".parse()?,
                router_address: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff".parse()?,
                pool_type: PoolType::QuickSwap,
                fee_percentage: Decimal::from_str("0.003")?,
            },
        ];
        
        Ok(Self {
            chain_manager,
            dex_configs,
        })
    }
    
    pub async fn get_known_pools(&self, chain: &Chain) -> Vec<LiquidityPool> {
        info!("Loading known pools for {:?}", chain);
        
        // Return some hardcoded pools for testing
        match chain {
            Chain::Ethereum => vec![
                LiquidityPool {
                    address: "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc".to_string(),
                    token0: TokenInfo {
                        address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                        symbol: "USDC".to_string(),
                        decimals: 6,
                        price_usd: Some(Decimal::from(1)),
                    },
                    token1: TokenInfo {
                        address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                        symbol: "WETH".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from(2000)),
                    },
                    reserve0: Decimal::from(100000000),
                    reserve1: Decimal::from(50000),
                    fee: Decimal::from_str("0.003").unwrap(),
                    exchange: "Uniswap V2".to_string(),
                    chain: Chain::Ethereum,
                    pool_type: PoolType::UniswapV2,
                    last_update: Utc::now(),
                },
                LiquidityPool {
                    address: "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".to_string(),
                    token0: TokenInfo {
                        address: "0xdAC17F958D2ee523a2206206994597C13D831ec7".to_string(),
                        symbol: "USDT".to_string(),
                        decimals: 6,
                        price_usd: Some(Decimal::from(1)),
                    },
                    token1: TokenInfo {
                        address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                        symbol: "WETH".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from(2000)),
                    },
                    reserve0: Decimal::from(150000000),
                    reserve1: Decimal::from(75000),
                    fee: Decimal::from_str("0.003").unwrap(),
                    exchange: "Uniswap V2".to_string(),
                    chain: Chain::Ethereum,
                    pool_type: PoolType::UniswapV2,
                    last_update: Utc::now(),
                },
                LiquidityPool {
                    address: "0xA478c2975Ab1Ea89e8196811F51A7B7Ade33eB11".to_string(),
                    token0: TokenInfo {
                        address: "0x6B175474E89094C44Da98b954EedeAC495271d0F".to_string(),
                        symbol: "DAI".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from(1)),
                    },
                    token1: TokenInfo {
                        address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                        symbol: "WETH".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from(2000)),
                    },
                    reserve0: Decimal::from(200000000),
                    reserve1: Decimal::from(100000),
                    fee: Decimal::from_str("0.003").unwrap(),
                    exchange: "Uniswap V2".to_string(),
                    chain: Chain::Ethereum,
                    pool_type: PoolType::UniswapV2,
                    last_update: Utc::now(),
                },
            ],
            Chain::BinanceSmartChain => vec![
                LiquidityPool {
                    address: "0x16b9a82891338f9bA80E2D6970FddA79D1eb0daE".to_string(),
                    token0: TokenInfo {
                        address: "0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c".to_string(),
                        symbol: "WBNB".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from(300)),
                    },
                    token1: TokenInfo {
                        address: "0x55d398326f99059fF775485246999027B3197955".to_string(),
                        symbol: "USDT".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from(1)),
                    },
                    reserve0: Decimal::from(100000),
                    reserve1: Decimal::from(30000000),
                    fee: Decimal::from_str("0.0025").unwrap(),
                    exchange: "PancakeSwap V2".to_string(),
                    chain: Chain::BinanceSmartChain,
                    pool_type: PoolType::PancakeV2,
                    last_update: Utc::now(),
                },
            ],
            Chain::Polygon => vec![
                LiquidityPool {
                    address: "0x604229c960e5CACF2aaEAc8Be68Ac07BA9dF81c3".to_string(),
                    token0: TokenInfo {
                        address: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".to_string(),
                        symbol: "WMATIC".to_string(),
                        decimals: 18,
                        price_usd: Some(Decimal::from_str("0.8").unwrap()),
                    },
                    token1: TokenInfo {
                        address: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
                        symbol: "USDC".to_string(),
                        decimals: 6,
                        price_usd: Some(Decimal::from(1)),
                    },
                    reserve0: Decimal::from(5000000),
                    reserve1: Decimal::from(4000000),
                    fee: Decimal::from_str("0.003").unwrap(),
                    exchange: "QuickSwap".to_string(),
                    chain: Chain::Polygon,
                    pool_type: PoolType::QuickSwap,
                    last_update: Utc::now(),
                },
            ],
            _ => vec![],
        }
    }
    
    pub async fn calculate_swap_amount(
        &self,
        pool: &LiquidityPool,
        amount_in: Decimal,
        token_in_is_token0: bool,
    ) -> Result<Decimal> {
        let (reserve_in, reserve_out) = if token_in_is_token0 {
            (pool.reserve0, pool.reserve1)
        } else {
            (pool.reserve1, pool.reserve0)
        };
        
        let amount_in_with_fee = amount_in * (Decimal::from(1) - pool.fee);
        let numerator = amount_in_with_fee * reserve_out;
        let denominator = reserve_in + amount_in_with_fee;
        
        Ok(numerator / denominator)
    }
    
    pub fn get_dexs_for_chain(&self, chain: &Chain) -> Vec<&DexConfig> {
        self.dex_configs.iter()
            .filter(|config| config.chain == *chain)
            .collect()
    }
}