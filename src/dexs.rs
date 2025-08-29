use std::sync::Arc;
use ethers::prelude::*;
use anyhow::Result;
use rust_decimal::Decimal;
use std::str::FromStr;
use crate::types::{Chain, LiquidityPool, TokenInfo, PoolType};
use crate::chains::ChainManager;
use chrono::Utc;

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

#[allow(dead_code)]impl DexManager {
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
            DexConfig {
                name: "TraderJoe".to_string(),
                chain: Chain::Avalanche,
                factory_address: "0x9Ad6C38BE94206cA50bb0d90783181662f0Cfa10".parse()?,
                router_address: "0x60aE616a2155Ee3d9A68541Ba4544862310933d4".parse()?,
                pool_type: PoolType::TraderJoe,
                fee_percentage: Decimal::from_str("0.003")?,
            },
            DexConfig {
                name: "SpookySwap".to_string(),
                chain: Chain::Fantom,
                factory_address: "0x152eE697f2E276fA89E96742e9bB9aB1F2E61bE3".parse()?,
                router_address: "0xF491e7B69E4244ad4002BC14e878a34207E38c29".parse()?,
                pool_type: PoolType::SpookySwap,
                fee_percentage: Decimal::from_str("0.002")?,
            },
            DexConfig {
                name: "Camelot".to_string(),
                chain: Chain::Arbitrum,
                factory_address: "0x6EcCab422D763aC031210895C81787E87B43A652".parse()?,
                router_address: "0xc873fEcbd354f5A56E00E710B90EF4201db2448d".parse()?,
                pool_type: PoolType::Camelot,
                fee_percentage: Decimal::from_str("0.003")?,
            },
            DexConfig {
                name: "Velodrome".to_string(),
                chain: Chain::Optimism,
                factory_address: "0x25CbdDb98b35ab1FF77413456B31EC81A6B6B746".parse()?,
                router_address: "0x9c12939390052919aF3155f41Bf4160Fd3666A6f".parse()?,
                pool_type: PoolType::Velodrome,
                fee_percentage: Decimal::from_str("0.003")?,
            },
        ];
        
        Ok(Self {
            chain_manager,
            dex_configs,
        })
    }
    
    pub async fn get_pool_info(&self, chain: &Chain, pool_address: Address) -> Result<LiquidityPool> {
        let provider = self.chain_manager.get_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
        
        let pool_abi = ethers::abi::parse_abi(&[
            "function token0() view returns (address)",
            "function token1() view returns (address)",
            "function getReserves() view returns (uint112 reserve0, uint112 reserve1, uint32 timestamp)",
            "function fee() view returns (uint24)",
        ])?;
        
        let pool = Contract::new(pool_address, pool_abi, provider.clone());
        
        let token0: Address = pool.method("token0", ())?.call().await?;
        let token1: Address = pool.method("token1", ())?.call().await?;
        let reserves: (U256, U256, u32) = pool.method("getReserves", ())?.call().await?;
        
        let fee = match pool.method::<_, U256>("fee", ())?.call().await {
            Ok(f) => Decimal::from_str(&f.to_string())? / Decimal::from(1_000_000),
            Err(_) => Decimal::from_str("0.003")?,
        };
        
        Ok(LiquidityPool {
            address: format!("{:?}", pool_address),
            token0: self.get_token_info(chain, token0).await?,
            token1: self.get_token_info(chain, token1).await?,
            reserve0: Decimal::from_str(&reserves.0.to_string())?,
            reserve1: Decimal::from_str(&reserves.1.to_string())?,
            fee,
            exchange: self.identify_dex(chain, pool_address).await?,
            chain: chain.clone(),
            pool_type: PoolType::UniswapV2,
            last_update: Utc::now(),
        })
    }
    
    async fn get_token_info(&self, chain: &Chain, token_address: Address) -> Result<TokenInfo> {
        let provider = self.chain_manager.get_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
        
        let token_abi = ethers::abi::parse_abi(&[
            "function symbol() view returns (string)",
            "function decimals() view returns (uint8)",
        ])?;
        
        let token = Contract::new(token_address, token_abi, provider);
        
        let symbol: String = token.method("symbol", ())?.call().await.unwrap_or_else(|_| "UNKNOWN".to_string());
        let decimals: u8 = token.method("decimals", ())?.call().await.unwrap_or(18);
        
        Ok(TokenInfo {
            address: format!("{:?}", token_address),
            symbol,
            decimals,
            price_usd: None,
        })
    }
    
    async fn identify_dex(&self, chain: &Chain, _pool_address: Address) -> Result<String> {
        for config in &self.dex_configs {
            if config.chain == *chain {
                return Ok(config.name.clone());
            }
        }
        Ok("Unknown".to_string())
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
    
    pub async fn get_all_pools(&self, chain: &Chain) -> Result<Vec<Address>> {
        let provider = self.chain_manager.get_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
        
        let mut all_pools = Vec::new();
        
        for config in self.get_dexs_for_chain(chain) {
            let factory_abi = ethers::abi::parse_abi(&[
                "function allPairsLength() view returns (uint256)",
                "function allPairs(uint256) view returns (address)",
            ])?;
            
            let factory = Contract::new(config.factory_address, factory_abi, provider.clone());
            
            match factory.method::<_, U256>("allPairsLength", ())?.call().await {
                Ok(length) => {
                    let pairs_to_fetch = length.as_u64().min(100);
                    for i in 0..pairs_to_fetch {
                        if let Ok(pool_address) = factory.method::<_, Address>("allPairs", i)?.call().await {
                            all_pools.push(pool_address);
                        }
                    }
                }
                Err(e) => {
                    tracing::debug!("Failed to get pools from {}: {}", config.name, e);
                }
            }
        }
        
        Ok(all_pools)
    }
}