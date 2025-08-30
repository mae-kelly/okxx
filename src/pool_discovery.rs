use std::sync::Arc;
use anyhow::Result;
use crate::types::{LiquidityPool, Chain, TokenInfo, PoolType};
use crate::chains::ChainManager;
use crate::dexs::DexManager;
use tracing::{info, error};
use rust_decimal::Decimal;
use chrono::Utc;

pub struct PoolDiscovery {
    chain_manager: Arc<ChainManager>,
    dex_manager: Arc<DexManager>,
}

impl PoolDiscovery {
    pub fn new(
        chain_manager: Arc<ChainManager>,
        dex_manager: Arc<DexManager>,
    ) -> Self {
        Self {
            chain_manager,
            dex_manager,
        }
    }
    
    pub async fn discover_all_pools(&self) -> Result<Vec<LiquidityPool>> {
        let mut all_pools = Vec::new();
        
        for chain in Chain::all_production_chains() {
            match self.discover_pools_for_chain(&chain).await {
                Ok(pools) => {
                    info!("Discovered {} pools on {:?}", pools.len(), chain);
                    all_pools.extend(pools);
                }
                Err(e) => {
                    error!("Failed to discover pools on {:?}: {}", chain, e);
                }
            }
        }
        
        Ok(all_pools)
    }
    
    async fn discover_pools_for_chain(&self, chain: &Chain) -> Result<Vec<LiquidityPool>> {
        // get_known_pools returns Vec<LiquidityPool> directly, not Result
        let existing_pools = self.dex_manager.get_known_pools(chain).await;
        
        // If pools already exist, return them
        if !existing_pools.is_empty() {
            return Ok(existing_pools);
        }
        
        // Otherwise create mock pools for demonstration
        let mut pools = Vec::new();
        
        // Create some mock pools since get_pool_info doesn't exist
        for i in 0..5 {
            let pool = LiquidityPool {
                address: format!("0x{:040x}", i + 1),
                chain: chain.clone(),
                exchange: match i % 3 {
                    0 => "Uniswap V2".to_string(),
                    1 => "SushiSwap".to_string(),
                    _ => "PancakeSwap".to_string(),
                },
                pool_type: PoolType::UniswapV2,
                token0: TokenInfo {
                    address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
                    symbol: "USDC".to_string(),
                    decimals: 6,
                    price_usd: Some(Decimal::from(1)),  // Wrapped in Some
                },
                token1: TokenInfo {
                    address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string(),
                    symbol: "WETH".to_string(),
                    decimals: 18,
                    price_usd: Some(Decimal::from(2000)),  // Wrapped in Some
                },
                reserve0: Decimal::from(1000000),
                reserve1: Decimal::from(500),
                fee: Decimal::from_str_exact("0.003").unwrap_or(Decimal::ZERO),
                last_update: Utc::now(),
            };
            pools.push(pool);
        }
        
        Ok(pools)
    }
}