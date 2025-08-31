use crate::config::Config;
use crate::types::{Chain, GasPrice};
use anyhow::Result;
use ethers::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;
use rust_decimal::Decimal;
use chrono::Utc;

pub struct ChainManager {
    providers: HashMap<Chain, Arc<Provider<Http>>>,
}

impl ChainManager {
    pub async fn new(config: &Config) -> Result<Self> {
        let mut providers = HashMap::new();
        
        // Initialize Ethereum provider
        if let Some(chain_config) = config.chains.get("ethereum") {
            if chain_config.enabled {
                let provider = Provider::<Http>::try_from(&chain_config.rpc_url)?;
                providers.insert(Chain::Ethereum, Arc::new(provider));
            }
        }
        
        // Initialize BSC provider
        if let Some(chain_config) = config.chains.get("bsc") {
            if chain_config.enabled {
                let provider = Provider::<Http>::try_from(&chain_config.rpc_url)?;
                providers.insert(Chain::BinanceSmartChain, Arc::new(provider));
            }
        }
        
        // Initialize Polygon provider
        if let Some(chain_config) = config.chains.get("polygon") {
            if chain_config.enabled {
                let provider = Provider::<Http>::try_from(&chain_config.rpc_url)?;
                providers.insert(Chain::Polygon, Arc::new(provider));
            }
        }
        
        // Initialize Arbitrum provider
        if let Some(chain_config) = config.chains.get("arbitrum") {
            if chain_config.enabled {
                let provider = Provider::<Http>::try_from(&chain_config.rpc_url)?;
                providers.insert(Chain::Arbitrum, Arc::new(provider));
            }
        }
        
        Ok(Self { providers })
    }
    
    pub fn get_provider(&self, chain: &Chain) -> Option<Arc<Provider<Http>>> {
        self.providers.get(chain).cloned()
    }
    
    pub async fn get_gas_price(&self, chain: &Chain) -> Result<GasPrice> {
        let provider = self.get_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found for chain {:?}", chain))?;
        
        let gas_price = provider.get_gas_price().await?;
        let gas_price_gwei = ethers::utils::format_units(gas_price, "gwei")?;
        let gas_decimal = Decimal::from_str_exact(&gas_price_gwei)?;
        
        Ok(GasPrice {
            chain: *chain,
            fast: gas_decimal * Decimal::from_str_exact("1.2")?,
            standard: gas_decimal,
            slow: gas_decimal * Decimal::from_str_exact("0.8")?,
            timestamp: Utc::now(),
        })
    }
    
    pub async fn get_block_number(&self, chain: &Chain) -> Result<u64> {
        let provider = self.get_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
        
        let block_number = provider.get_block_number().await?;
        Ok(block_number.as_u64())
    }
    
    pub async fn get_eth_price(&self) -> Result<Decimal> {
        // In production, fetch from oracle or price feed
        // For now, return a static value
        Ok(Decimal::from(2500))
    }
}

use rust_decimal::prelude::FromStr;