use std::sync::Arc;
use tokio::sync::RwLock;
use dashmap::DashMap;
use tracing::{info, error};
use tracing_subscriber;

mod types;
mod chains;
mod dexs;
mod websocket;
mod arbitrage;
mod ml;
mod storage;
mod metrics;

use types::*;
use chains::ChainManager;
use dexs::DexManager;
use websocket::WebSocketManager;
use arbitrage::ArbitrageEngine;
use ml::MLAnalyzer;
use storage::StorageEngine;
use metrics::MetricsServer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    
    let shared_state = Arc::new(SharedState {
        prices: Arc::new(DashMap::new()),
        liquidity_pools: Arc::new(DashMap::new()),
        gas_prices: Arc::new(DashMap::new()),
        opportunities: Arc::new(RwLock::new(Vec::new())),
        historical_data: Arc::new(RwLock::new(Vec::new())),
    });
    
    let storage = Arc::new(StorageEngine::new("./data")?);
    let chain_manager = Arc::new(ChainManager::new().await?);
    let dex_manager = Arc::new(DexManager::new(chain_manager.clone()).await?);
    let ws_manager = Arc::new(WebSocketManager::new(shared_state.clone()).await?);
    let arbitrage_engine = Arc::new(ArbitrageEngine::new(
        shared_state.clone(),
        chain_manager.clone(),
        dex_manager.clone(),
    ));
    let ml_analyzer = Arc::new(MLAnalyzer::new(storage.clone())?);
    let metrics_server = MetricsServer::new(8080);
    
    let _state_clone = shared_state.clone();
    let ws_clone = ws_manager.clone();
    tokio::spawn(async move {
        ws_clone.start_all_connections().await;
    });
    
    let arb_clone = arbitrage_engine.clone();
    let state_clone2 = shared_state.clone();
    let storage_clone = storage.clone();
    tokio::spawn(async move {
        loop {
            match arb_clone.scan_opportunities().await {
                Ok(opportunities) => {
                    let mut opps = state_clone2.opportunities.write().await;
                    *opps = opportunities.clone();
                    
                    for opp in &opportunities {
                        if opp.profit_usd > 100.0 {
                            info!(
                                "Arbitrage found: {} -> {} | Profit: ${:.2} | ROI: {:.2}%",
                                opp.path[0].exchange,
                                opp.path[opp.path.len() - 1].exchange,
                                opp.profit_usd,
                                opp.roi_percentage
                            );
                            
                            let _ = storage_clone.store_opportunity(opp).await;
                        }
                    }
                }
                Err(e) => error!("Arbitrage scan error: {}", e),
            }
            
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
        }
    });
    
    let ml_clone = ml_analyzer.clone();
    let storage_clone2 = storage.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            
            match storage_clone2.get_recent_opportunities(1000).await {
                Ok(data) => {
                    if data.len() > 100 {
                        match ml_clone.analyze_patterns(&data).await {
                            Ok(insights) => {
                                info!("ML Insights generated: {:?}", insights);
                                let _ = storage_clone2.store_ml_insights(&insights).await;
                            }
                            Err(e) => error!("ML analysis error: {}", e),
                        }
                    }
                }
                Err(e) => error!("Failed to fetch historical data: {}", e),
            }
        }
    });
    
    tokio::spawn(async move {
        metrics_server.run().await;
    });
    
    info!("Arbitrage scanner started. Monitoring all chains and DEXs...");
    
    tokio::signal::ctrl_c().await?;
    info!("Shutting down...");
    
    Ok(())
}