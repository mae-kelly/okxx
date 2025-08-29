use ethers::providers::{Provider, Http};
// Ws provider removed - using Http instead
use ethers::types::transaction::eip2718::TypedTransaction;use std::sync::Arc;
use ethers::prelude::*;
use anyhow::Result;
use dashmap::DashMap;
use rust_decimal::Decimal;
use std::str::FromStr;
use crate::types::{Chain, GasPrice};
use chrono::Utc;

pub struct ChainManager {
    providers: DashMap<Chain, Arc<Provider<Http>>>,
    websocket_providers: DashMap<Chain, Arc<Provider<Http>>>,
}

#[allow(dead_code)]impl ChainManager {
    pub async fn new() -> Result<Self> {
        let manager = Self {
            providers: DashMap::new(),
            websocket_providers: DashMap::new(),
        };
        
        manager.initialize_providers().await?;
        Ok(manager)
    }
    
    async fn initialize_providers(&self) -> Result<()> {
        let chains = vec![
            (Chain::Ethereum, "https://eth-mainnet.g.alchemy.com/v2/demo", "wss://eth-mainnet.g.alchemy.com/v2/demo"),
            (Chain::BinanceSmartChain, "https://bsc-dataseed.binance.org", "wss://bsc-ws-node.nariox.org:443"),
            (Chain::Polygon, "https://polygon-rpc.com", "wss://polygon-bor.publicnode.com"),
            (Chain::Arbitrum, "https://arb1.arbitrum.io/rpc", "wss://arb1.arbitrum.io/ws"),
            (Chain::Optimism, "https://mainnet.optimism.io", "wss://ws-mainnet.optimism.io"),
            (Chain::Avalanche, "https://api.avax.network/ext/bc/C/rpc", "wss://api.avax.network/ext/bc/C/ws"),
            (Chain::Fantom, "https://rpc.ftm.tools", "wss://wsapi.fantom.network"),
            (Chain::Base, "https://mainnet.base.org", "wss://base-mainnet.publicnode.com"),
            (Chain::ZkSync, "https://mainnet.era.zksync.io", "wss://mainnet.era.zksync.io/ws"),
            (Chain::Linea, "https://rpc.linea.build", "wss://rpc.linea.build/ws"),
            (Chain::Scroll, "https://rpc.scroll.io", "wss://rpc.scroll.io/ws"),
            (Chain::Blast, "https://rpc.blast.io", "wss://rpc.blast.io"),
        ];
        
        for (chain, rpc_url, ws_url) in chains {
            match Provider::<Http>::try_from(rpc_url) {
                Ok(provider) => {
                    self.providers.insert(chain.clone(), Arc::new(provider));
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to {} RPC: {}", rpc_url, e);
                }
            }
            
            match Provider::<Http>::try_from(ws_url.replace("ws", "http").replace("wss", "https").as_str()) {
                Ok(provider) => {
                    self.websocket_providers.insert(chain.clone(), Arc::new(provider));
                }
                Err(e) => {
                    tracing::warn!("Failed to connect to {} WebSocket: {}", ws_url, e);
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn get_gas_price(&self, chain: &Chain) -> Result<GasPrice> {
        let provider = self.providers.get(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found for chain {:?}", chain))?;
        
        let gas_price = provider.get_gas_price().await?;
        let base_fee = provider.get_block(BlockNumber::Latest)
            .await?
            .and_then(|b| b.base_fee_per_gas)
            .unwrap_or(gas_price);
        
        let fast = Decimal::from_str(&ethers::utils::format_units(gas_price * 120 / 100, "gwei")?)?;
        let standard = Decimal::from_str(&ethers::utils::format_units(gas_price, "gwei")?)?;
        let slow = Decimal::from_str(&ethers::utils::format_units(gas_price * 80 / 100, "gwei")?)?;
        let base = Decimal::from_str(&ethers::utils::format_units(base_fee, "gwei")?)?;
        let priority = fast - base;
        
        Ok(GasPrice {
            chain: chain.clone(),
            fast,
            standard,
            slow,
            base_fee: base,
            priority_fee: priority,
            timestamp: Utc::now(),
        })
    }
    
    pub async fn estimate_gas(&self, chain: &Chain, data: Vec<u8>) -> Result<U256> {
        let provider = self.providers.get(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
        
        let tx = TypedTransaction::Legacy(TransactionRequest::new()
            .data(data)
            .to(Address::zero()));
        
        let gas = provider.estimate_gas(&tx, None).await?;
        Ok(gas)
    }
    
    pub fn get_provider(&self, chain: &Chain) -> Option<Arc<Provider<Http>>> {
        self.providers.get(chain).map(|p| p.clone())
    }
    
    pub fn get_ws_provider(&self, chain: &Chain) -> Option<Arc<Provider<Http>>> {
        self.websocket_providers.get(chain).map(|p| p.clone())
    }
    
    pub async fn get_block_number(&self, chain: &Chain) -> Result<u64> {
        let provider = self.providers.get(chain)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;
        
        let block = provider.get_block_number().await?;
        Ok(block.as_u64())
    }
    
//     pub async fn subscribe_blocks(&self, chain: &Chain) -> Result<()> {
//         if let Some(ws_provider) = self.websocket_providers.get(chain) {
//             let mut stream = ws_provider.subscribe_blocks().await?;
}
