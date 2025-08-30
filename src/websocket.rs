use std::sync::Arc;
use anyhow::Result;
use ethers::prelude::*;
use ethers::providers::{Provider, Http, Middleware};
use ethers::types::{H256, U256, Filter, U64, BlockId, BlockNumber};
use tokio::sync::RwLock;
use tracing::{info, error, debug, warn};
use crate::types::{SharedState, Chain, GasPrice, LiquidityPool, PoolType, TokenInfo};
use rust_decimal::Decimal;
use std::str::FromStr;
use chrono::Utc;

// Common DEX pool events to monitor
const UNISWAP_V2_SYNC_TOPIC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";
const UNISWAP_V2_SWAP_TOPIC: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";

pub struct WebSocketManager {
    state: Arc<SharedState>,
    providers: Arc<RwLock<Vec<(Chain, Arc<Provider<Http>>)>>>,
}

impl WebSocketManager {
    pub async fn new(state: Arc<SharedState>) -> Result<Self> {
        Ok(Self { 
            state,
            providers: Arc::new(RwLock::new(Vec::new())),
        })
    }
    
    pub async fn start_all_connections(&self) {
        // Use your actual Alchemy API key
        let alchemy_key = "alcht_oZ7wU7JpIoZejlOWUcMFOpNsIlLDsX";
        
        info!("Starting blockchain connections...");
        info!("To enable Polygon, visit: https://dashboard.alchemy.com/apps/gj611zq6c8gqf1kk/networks");
        
        // Connect to Ethereum mainnet
        let eth_manager = self.clone();
        let eth_key = alchemy_key.to_string();
        tokio::spawn(async move {
            if let Err(e) = eth_manager.connect_ethereum(&eth_key).await {
                error!("Failed to connect to Ethereum: {}", e);
            }
        });
        
        // Connect to Arbitrum
        let arb_manager = self.clone();
        let arb_key = alchemy_key.to_string();
        tokio::spawn(async move {
            if let Err(e) = arb_manager.connect_arbitrum(&arb_key).await {
                error!("Failed to connect to Arbitrum: {}", e);
            }
        });
        
        // Skip Polygon for now since it's not enabled
        warn!("Skipping Polygon - network not enabled in Alchemy dashboard");
        warn!("Enable it at: https://dashboard.alchemy.com/apps/gj611zq6c8gqf1kk/networks");
    }
    
    async fn connect_ethereum(&self, api_key: &str) -> Result<()> {
        let rpc_url = format!("https://eth-mainnet.g.alchemy.com/v2/{}", api_key);
        let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        
        // Test connection
        match provider.get_block_number().await {
            Ok(block) => {
                info!("✓ Connected to Ethereum - Block #{}", block);
                
                // Store provider
                let mut providers = self.providers.write().await;
                providers.push((Chain::Ethereum, provider.clone()));
                drop(providers);
                
                // Start monitoring
                self.monitor_chain(Chain::Ethereum, provider).await?;
            }
            Err(e) => {
                error!("Failed to connect to Ethereum: {}", e);
                return Err(anyhow::anyhow!("Ethereum connection failed: {}", e));
            }
        }
        
        Ok(())
    }
    
    async fn connect_arbitrum(&self, api_key: &str) -> Result<()> {
        let rpc_url = format!("https://arb-mainnet.g.alchemy.com/v2/{}", api_key);
        let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        
        // Test connection
        match provider.get_block_number().await {
            Ok(block) => {
                info!("✓ Connected to Arbitrum - Block #{}", block);
                
                // Store provider
                let mut providers = self.providers.write().await;
                providers.push((Chain::Arbitrum, provider.clone()));
                drop(providers);
                
                // Start monitoring
                self.monitor_chain(Chain::Arbitrum, provider).await?;
            }
            Err(e) => {
                if e.to_string().contains("not enabled") {
                    warn!("Arbitrum network not enabled in Alchemy");
                    warn!("Enable it at: https://dashboard.alchemy.com/apps/gj611zq6c8gqf1kk/networks");
                } else {
                    error!("Failed to connect to Arbitrum: {}", e);
                }
                return Err(anyhow::anyhow!("Arbitrum connection failed: {}", e));
            }
        }
        
        Ok(())
    }
    
    async fn connect_polygon(&self, api_key: &str) -> Result<()> {
        let rpc_url = format!("https://polygon-mainnet.g.alchemy.com/v2/{}", api_key);
        let provider = Arc::new(Provider::<Http>::try_from(rpc_url)?);
        
        // Test connection
        match provider.get_block_number().await {
            Ok(block) => {
                info!("✓ Connected to Polygon - Block #{}", block);
                
                // Store provider
                let mut providers = self.providers.write().await;
                providers.push((Chain::Polygon, provider.clone()));
                drop(providers);
                
                // Start monitoring
                self.monitor_chain(Chain::Polygon, provider).await?;
            }
            Err(e) => {
                if e.to_string().contains("not enabled") {
                    warn!("Polygon network not enabled in Alchemy");
                    warn!("Enable it at: https://dashboard.alchemy.com/apps/gj611zq6c8gqf1kk/networks");
                } else {
                    error!("Failed to connect to Polygon: {}", e);
                }
                return Err(anyhow::anyhow!("Polygon connection failed: {}", e));
            }
        }
        
        Ok(())
    }
    
    async fn monitor_chain(&self, chain: Chain, provider: Arc<Provider<Http>>) -> Result<()> {
        let state = self.state.clone();
        
        // Single monitoring task per chain
        tokio::spawn(async move {
            let mut last_block = 0u64;
            let mut consecutive_errors = 0;
            
            info!("Starting monitor for {:?}", chain);
            
            loop {
                // Rate limiting - adjust based on chain
                let sleep_duration = match chain {
                    Chain::Ethereum => 12, // ~5 blocks per minute
                    Chain::Arbitrum => 2,  // Faster blocks
                    Chain::Polygon => 3,   // 2-3 second blocks
                    _ => 10,
                };
                tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
                
                // Get block number and gas price
                match provider.get_block_number().await {
                    Ok(block_number) => {
                        consecutive_errors = 0;
                        let current_block = block_number.as_u64();
                        
                        if current_block > last_block {
                            // Only log every 10th block to reduce noise
                            if current_block % 10 == 0 {
                                info!("📦 {:?} Block #{}", chain, current_block);
                            }
                            last_block = current_block;
                            
                            // Get gas price
                            if let Ok(gas_price) = provider.get_gas_price().await {
                                let gas_price_gwei = gas_price.as_u128() as f64 / 1_000_000_000.0;
                                let base_fee_decimal = Decimal::from_str(&gas_price_gwei.to_string()).unwrap_or_default();
                                
                                let priority_fee = match chain {
                                    Chain::Ethereum => Decimal::from(2),
                                    Chain::Arbitrum => Decimal::from_str("0.1").unwrap(),
                                    Chain::Polygon => Decimal::from(30),
                                    _ => Decimal::from(1),
                                };
                                
                                let gas_price_struct = GasPrice {
                                    fast: base_fee_decimal + priority_fee * Decimal::from(2),
                                    standard: base_fee_decimal + priority_fee,
                                    slow: base_fee_decimal + priority_fee / Decimal::from(2),
                                    base_fee: base_fee_decimal,
                                    priority_fee,
                                    chain: chain.clone(),
                                    timestamp: Utc::now(),
                                };
                                
                                state.gas_prices.insert(chain.clone(), gas_price_struct);
                                
                                // Log gas price every 50 blocks
                                if current_block % 50 == 0 {
                                    info!("⛽ {:?} Gas: {:.2} gwei", chain, base_fee_decimal);
                                }
                            }
                            
                            // Check for DEX events every 20 blocks
                            if current_block % 20 == 0 {
                                let filter = Filter::new()
                                    .topic0(vec![
                                        H256::from_str(UNISWAP_V2_SYNC_TOPIC).unwrap(),
                                        H256::from_str(UNISWAP_V2_SWAP_TOPIC).unwrap(),
                                    ])
                                    .from_block(block_number.saturating_sub(U64::from(5)))
                                    .to_block(block_number);
                                
                                if let Ok(logs) = provider.get_logs(&filter).await {
                                    if !logs.is_empty() {
                                        info!("📊 Found {} DEX events on {:?}", logs.len(), chain);
                                    }
                                    
                                    for log in logs {
                                        // Process Swap events
                                        if log.topics[0] == H256::from_str(UNISWAP_V2_SWAP_TOPIC).unwrap() {
                                            if log.data.len() >= 128 {
                                                let amount0_in = U256::from_big_endian(&log.data[0..32]);
                                                let amount1_in = U256::from_big_endian(&log.data[32..64]);
                                                
                                                let swap_size = amount0_in.max(amount1_in);
                                                // Check for large swaps (> 10 ETH)
                                                if swap_size > U256::from_dec_str("10000000000000000000").unwrap() {
                                                    let size_eth = swap_size.as_u128() as f64 / 1e18;
                                                    warn!("🔥 LARGE SWAP on {:?}: {:.2} ETH", chain, size_eth);
                                                    warn!("   Pool: {:?}", log.address);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        
                        if e.to_string().contains("not enabled") {
                            error!("{:?} network not enabled in Alchemy. Stopping monitor.", chain);
                            break;
                        }
                        
                        if consecutive_errors > 10 {
                            error!("Too many consecutive errors on {:?}. Stopping monitor.", chain);
                            break;
                        }
                        
                        error!("Error on {:?}: {} (attempt {})", chain, e, consecutive_errors);
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                    }
                }
            }
            
            warn!("Monitor stopped for {:?}", chain);
        });
        
        Ok(())
    }
}

impl Clone for WebSocketManager {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            providers: self.providers.clone(),
        }
    }
}