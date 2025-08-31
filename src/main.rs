use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use tracing::{info, error};
use anyhow::Result;

mod types;
mod config;
mod chains;
mod dexs;
mod flashloan;
mod arbitrage;
mod websocket;
mod storage;

use types::*;
use config::Config;
use chains::ChainManager;
use dexs::DexManager;
use flashloan::FlashLoanManager;
use arbitrage::ArbitrageEngine;
use websocket::PriceMonitor;
use storage::StorageEngine;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(true)
        .init();

    info!("🚀 Starting DeFi Arbitrage Scanner with Flash Loan Support");

    // Load configuration
    let config = Config::load()?;
    
    // Initialize shared state
    let state = Arc::new(SharedState {
        prices: Arc::new(DashMap::new()),
        pools: Arc::new(DashMap::new()),
        gas_prices: Arc::new(DashMap::new()),
        opportunities: Arc::new(RwLock::new(Vec::new())),
    });

    // Initialize storage
    let storage = Arc::new(StorageEngine::new("./data")?);
    
    // Initialize chain manager
    let chain_manager = Arc::new(ChainManager::new(&config).await?);
    
    // Initialize DEX manager
    let dex_manager = Arc::new(DexManager::new(chain_manager.clone()).await?);
    
    // Initialize flash loan manager
    let flash_loan_manager = Arc::new(FlashLoanManager::new(&config, chain_manager.clone()).await?);
    
    // Initialize arbitrage engine
    let arbitrage_engine = Arc::new(ArbitrageEngine::new(
        state.clone(),
        chain_manager.clone(),
        dex_manager.clone(),
        flash_loan_manager.clone(),
        config.clone(),
    ));
    
    // Initialize price monitor
    let price_monitor = Arc::new(PriceMonitor::new(state.clone()));
    
    // Start price monitoring
    let monitor_clone = price_monitor.clone();
    tokio::spawn(async move {
        if let Err(e) = monitor_clone.start().await {
            error!("Price monitor error: {}", e);
        }
    });
    
    // Start arbitrage scanning loop
    let arb_engine = arbitrage_engine.clone();
    let state_clone = state.clone();
    let storage_clone = storage.clone();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        
        loop {
            interval.tick().await;
            
            match arb_engine.scan_opportunities().await {
                Ok(opportunities) => {
                    let mut opps = state_clone.opportunities.write().await;
                    *opps = opportunities.clone();
                    
                    for opp in opportunities {
                        if opp.net_profit_usd > config.min_profit_usd {
                            info!(
                                "💰 Arbitrage Opportunity Found!
                                Chain: {:?}
                                Type: {}
                                Profit: ${:.2}
                                ROI: {:.2}%
                                Flash Loan: {} ({:.3}% fee)
                                Gas Cost: ${:.2}",
                                opp.chain,
                                opp.opportunity_type,
                                opp.net_profit_usd,
                                opp.roi_percentage,
                                opp.flash_loan_provider,
                                opp.flash_loan_fee_percentage,
                                opp.gas_cost_usd
                            );
                            
                            // Store opportunity
                            let _ = storage_clone.store_opportunity(&opp).await;
                        }
                    }
                }
                Err(e) => error!("Scan error: {}", e),
            }
        }
    });
    
    // Start gas price monitoring
    let chain_mgr = chain_manager.clone();
    let state_clone2 = state.clone();
    
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(15));
        
        loop {
            interval.tick().await;
            
            for chain in Chain::all() {
                if let Ok(gas_price) = chain_mgr.get_gas_price(&chain).await {
                    state_clone2.gas_prices.insert(chain, gas_price);
                }
            }
        }
    });
    
    info!("✅ All systems initialized. Scanning for arbitrage opportunities...");
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    Ok(())
}