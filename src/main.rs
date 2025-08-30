use anyhow::Result;
use std::sync::Arc;
use tracing::{info, warn, error};
use tracing_subscriber;

mod types;
mod chains;
mod websocket;
mod arbitrage;
mod flashloan;
mod ml;
mod pool_discovery;
mod storage;
mod dexs;

use crate::{
    types::SharedState,
    chains::ChainManager,
    websocket::WebSocketManager,
    arbitrage::ArbitrageEngine,
    flashloan::FlashLoanManager,
    ml::MLAnalyzer,
    pool_discovery::PoolDiscovery,
    storage::StorageEngine,
    dexs::DexManager,
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(false)
        .init();

    info!("========================================");
    info!("   ARBITRAGE SCANNER v2.0 STARTING     ");
    info!("========================================");
    
    // Create shared state
    let state = Arc::new(SharedState::default());
    info!("✓ Shared state initialized");
    
    // Initialize chain manager
    let chain_manager = Arc::new(ChainManager::new().await?);
    info!("✓ Chain manager initialized");
    
    // Initialize storage
    let storage = Arc::new(StorageEngine::new("./data/arbitrage.db")?);
    info!("✓ Storage engine initialized");
    
    // Initialize ML analyzer
    let ml_analyzer = Arc::new(MLAnalyzer::new(storage.clone())?);
    info!("✓ ML analyzer initialized");
    
    // Initialize DEX manager - await the async call
    let dex_manager = Arc::new(DexManager::new(chain_manager.clone()).await?);
    info!("✓ DEX manager initialized");
    
    // Initialize flash loan manager
    let flashloan_manager = Arc::new(FlashLoanManager::new(chain_manager.clone()));
    info!("✓ Flash loan manager initialized");
    
    // Initialize pool discovery
    let pool_discovery = Arc::new(PoolDiscovery::new(
        chain_manager.clone(),
        dex_manager.clone()
    ));
    info!("✓ Pool discovery initialized");
    
    // Initialize arbitrage engine
    let arbitrage_engine = Arc::new(ArbitrageEngine::new(
        state.clone(),
        chain_manager.clone(),
        dex_manager.clone()
    ));
    info!("✓ Arbitrage engine initialized");
    
    // Initialize WebSocket manager and start connections
    let ws_manager = WebSocketManager::new(state.clone()).await?;
    info!("✓ WebSocket manager initialized");
    
    info!("----------------------------------------");
    info!("Starting blockchain connections...");
    info!("----------------------------------------");
    
    // Check for API key
    let api_key = std::env::var("ALCHEMY_API_KEY")
        .unwrap_or_else(|_| {
            warn!("ALCHEMY_API_KEY not found in environment");
            warn!("Using hardcoded key from source");
            "alcht_oZ7wU7JpIoZejlOWUcMFOpNsIlLDsX".to_string()
        });
    
    if api_key == "demo" || api_key.is_empty() {
        error!("Invalid API key. Please set ALCHEMY_API_KEY environment variable");
        return Ok(());
    }
    
    info!("Using Alchemy API key: {}...", &api_key[..10]);
    
    // Start WebSocket connections
    ws_manager.start_all_connections().await;
    
    // Give connections time to establish
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    info!("========================================");
    info!("    SCANNER RUNNING - MONITORING...    ");
    info!("========================================");
    info!("");
    info!("Watching for:");
    info!("  • Large swaps (>10 ETH)");
    info!("  • Gas price changes");
    info!("  • Arbitrage opportunities");
    info!("");
    info!("Press Ctrl+C to stop");
    info!("");
    
    // Keep the main thread alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
        
        // Print status every minute
        let gas_prices = state.gas_prices.len();
        let pools = state.liquidity_pools.len();
        let opportunities = state.opportunities.read().await.len();
        
        if gas_prices > 0 || pools > 0 || opportunities > 0 {
            info!("📊 Status: {} chains | {} pools | {} opportunities", 
                  gas_prices, pools, opportunities);
        }
    }
}