// rust-engine/src/advanced_scanner.rs
use ethers::prelude::*;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};
use anyhow::Result;
use tokio::sync::RwLock;
use dashmap::DashMap;
use futures::StreamExt;

use crate::config::{ChainConfig, DexConfig};

pub struct AdvancedScanner {
    provider: Arc<Provider<Http>>,
    ws_provider: Option<Arc<Provider<Ws>>>,
    config: ChainConfig,
    all_pairs: Arc<RwLock<HashMap<String, Vec<PairData>>>>,
    pair_update_times: Arc<DashMap<String, u64>>,
    all_tokens: Arc<RwLock<HashSet<Address>>>,
}

#[derive(Clone, Debug)]
struct PairData {
    dex_name: String,
    factory: Address,
    pair_address: Address,
    token0: Address,
    token1: Address,
    reserves: (U256, U256),
    last_update: u64,
}

impl AdvancedScanner {
    pub async fn new(
        provider: Arc<Provider<Http>>,
        ws_url: Option<String>,
        config: ChainConfig,
    ) -> Result<Self> {
        let ws_provider = if let Some(url) = ws_url {
            match Provider::<Ws>::connect(url).await {
                Ok(p) => Some(Arc::new(p)),
                Err(_) => None,
            }
        } else {
            None
        };
        
        Ok(Self {
            provider,
            ws_provider,
            config,
            all_pairs: Arc::new(RwLock::new(HashMap::new())),
            pair_update_times: Arc::new(DashMap::new()),
            all_tokens: Arc::new(RwLock::new(HashSet::new())),
        })
    }
    
    pub async fn discover_all_pairs(&self) -> Result<()> {
        println!("🔎 Discovering all pairs on {}...", self.config.name);
        
        for dex in &self.config.dexes {
            if let Err(e) = self.discover_dex_pairs(dex).await {
                println!("⚠️ Error discovering pairs for {}: {}", dex.name, e);
            }
        }
        
        let pairs = self.all_pairs.read().await;
        let tokens = self.all_tokens.read().await;
        println!("✅ Found {} unique pairs across {} tokens", pairs.len(), tokens.len());
        
        Ok(())
    }
    
    async fn discover_dex_pairs(&self, dex: &DexConfig) -> Result<()> {
        let factory_abi = ethers::abi::parse_abi(&[
            "function allPairsLength() view returns (uint)",
            "function allPairs(uint) view returns (address)",
        ])?;
        
        let factory = Contract::new(dex.factory, factory_abi, self.provider.clone());
        
        let length: U256 = match factory.method("allPairsLength", ())?.call().await {
            Ok(l) => l,
            Err(_) => {
                println!("  {} pairs discovery failed", dex.name);
                return Ok(());
            }
        };
        
        println!("  {} has {} pairs", dex.name, length);
        
        let batch_size = 20; // Reduced batch size
        let total_pairs = (length.as_u64() as usize).min(100); // Limit to first 100 pairs
        
        for i in (0..total_pairs).step_by(batch_size) {
            let end = (i + batch_size).min(total_pairs);
            let mut batch_futures = vec![];
            
            for j in i..end {
                let factory = factory.clone();
                let future = async move {
                    factory
                        .method::<_, Address>("allPairs", U256::from(j))?
                        .call()
                        .await
                };
                batch_futures.push(future);
            }
            
            let results = futures::future::join_all(batch_futures).await;
            
            for (_idx, result) in results.into_iter().enumerate() {
                if let Ok(pair_address) = result {
                    if let Ok(Some(pair_data)) = self.fetch_pair_details(pair_address, dex.name.clone()).await {
                        let key = format!("{:?}-{:?}", pair_data.token0, pair_data.token1);
                        
                        self.all_pairs.write().await
                            .entry(key.clone())
                            .or_insert_with(Vec::new)
                            .push(pair_data.clone());
                        
                        self.all_tokens.write().await.insert(pair_data.token0);
                        self.all_tokens.write().await.insert(pair_data.token1);
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        
        Ok(())
    }
    
    async fn fetch_pair_details(&self, pair_address: Address, dex_name: String) -> Result<Option<PairData>> {
        let pair_abi = ethers::abi::parse_abi(&[
            "function token0() view returns (address)",
            "function token1() view returns (address)",
            "function getReserves() view returns (uint112,uint112,uint32)",
        ])?;
        
        let pair = Contract::new(pair_address, pair_abi.clone(), self.provider.clone());
        
        // Execute calls sequentially to avoid lifetime issues
        let token0 = match pair.method::<_, Address>("token0", ())?.call().await {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        
        let token1 = match pair.method::<_, Address>("token1", ())?.call().await {
            Ok(t) => t,
            Err(_) => return Ok(None),
        };
        
        let reserves = match pair.method::<_, (U256, U256, U256)>("getReserves", ())?.call().await {
            Ok(r) => r,
            Err(_) => return Ok(None),
        };
        
        Ok(Some(PairData {
            dex_name,
            factory: Address::zero(),
            pair_address,
            token0,
            token1,
            reserves: (reserves.0, reserves.1),
            last_update: self.current_timestamp(),
        }))
    }
    
    pub async fn smart_update_reserves(&self) -> Result<()> {
        let now = self.current_timestamp();
        let stale_threshold = 5000;
        
        let pairs_to_update: Vec<_> = {
            let pairs = self.all_pairs.read().await;
            pairs.iter()
                .flat_map(|(_, dex_pairs)| dex_pairs.iter())
                .filter(|p| now - p.last_update > stale_threshold)
                .map(|p| p.pair_address)
                .take(10) // Limit updates to 10 at a time
                .collect()
        };
        
        for pair_addr in pairs_to_update {
            let _ = self.update_single_pair_reserves(pair_addr).await;
        }
        
        Ok(())
    }
    
    async fn update_single_pair_reserves(&self, pair_address: Address) -> Result<()> {
        let pair_abi = ethers::abi::parse_abi(&[
            "function getReserves() view returns (uint112,uint112,uint32)",
        ])?;
        
        let pair = Contract::new(pair_address, pair_abi, self.provider.clone());
        
        if let Ok(reserves) = pair.method::<_, (U256, U256, U256)>("getReserves", ())?.call().await {
            let mut pairs = self.all_pairs.write().await;
            for (_, dex_pairs) in pairs.iter_mut() {
                for pair_data in dex_pairs.iter_mut() {
                    if pair_data.pair_address == pair_address {
                        pair_data.reserves = (reserves.0, reserves.1);
                        pair_data.last_update = self.current_timestamp();
                    }
                }
            }
        }
        
        Ok(())
    }
    
    pub async fn find_all_opportunities(&self) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();
        let pairs = self.all_pairs.read().await;
        
        for (_token_pair, dex_pairs) in pairs.iter() {
            if dex_pairs.len() < 2 {
                continue;
            }
            
            let mut prices: Vec<(String, f64, &PairData)> = Vec::new();
            
            for pair_data in dex_pairs {
                if pair_data.reserves.0 > U256::zero() && pair_data.reserves.1 > U256::zero() {
                    let price = pair_data.reserves.0.as_u128() as f64 / 
                               pair_data.reserves.1.as_u128() as f64;
                    prices.push((pair_data.dex_name.clone(), price, pair_data));
                }
            }
            
            for i in 0..prices.len() {
                for j in i+1..prices.len() {
                    let (dex1, price1, data1) = &prices[i];
                    let (dex2, price2, data2) = &prices[j];
                    
                    let spread_pct = ((price1 - price2).abs() / price1.min(*price2)) * 100.0;
                    
                    if spread_pct > 0.3 {
                        opportunities.push(ArbitrageOpportunity {
                            token0: data1.token0,
                            token1: data1.token1,
                            dex1: dex1.clone(),
                            dex2: dex2.clone(),
                            pair1: data1.pair_address,
                            pair2: data2.pair_address,
                            spread_pct,
                            reserves1: data1.reserves,
                            reserves2: data2.reserves,
                        });
                    }
                }
            }
        }
        
        opportunities.sort_by(|a, b| b.spread_pct.partial_cmp(&a.spread_pct).unwrap());
        opportunities.truncate(20); // Return top 20 opportunities
        opportunities
    }
    
    pub async fn subscribe_to_updates(&self) -> Result<()> {
        if let Some(ws_provider) = &self.ws_provider {
            let ws_clone = ws_provider.clone();
            
            tokio::spawn(async move {
                if let Ok(mut stream) = ws_clone.subscribe_blocks().await {
                    while let Some(block) = stream.next().await {
                        println!("New block: {}", block.number.unwrap_or_default());
                    }
                }
            });
        }
        
        Ok(())
    }
    
    fn current_timestamp(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }
}

#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub token0: Address,
    pub token1: Address,
    pub dex1: String,
    pub dex2: String,
    pub pair1: Address,
    pub pair2: Address,
    pub spread_pct: f64,
    pub reserves1: (U256, U256),
    pub reserves2: (U256, U256),
}