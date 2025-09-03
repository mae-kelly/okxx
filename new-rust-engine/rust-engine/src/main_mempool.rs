use ethers::prelude::*;
use std::sync::Arc;
use tokio_stream::StreamExt;
use log::{info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("🔥 MEMPOOL MONITORING MODE");
    info!("Watching for transactions in real-time...");
    
    // Connect via WebSocket for real-time updates
    let ws = Ws::connect("wss://arbitrum-one.publicnode.com").await?;
    let provider = Arc::new(Provider::new(ws));
    
    info!("✅ WebSocket connected");
    info!("👀 Monitoring pending transactions...");
    
    // Subscribe to pending transactions
    let mut stream = provider.subscribe_pending_txs().await?;
    
    let mut tx_count = 0;
    
    while let Some(tx_hash) = stream.next().await {
        tx_count += 1;
        
        // Get transaction details fast
        if let Ok(Some(tx)) = provider.get_transaction(tx_hash).await {
            // Check if it's a DEX trade
            if is_dex_trade(&tx) {
                info!("🎯 DEX Trade detected!");
                info!("   Hash: {:?}", tx_hash);
                info!("   Gas: {} Gwei", tx.gas_price.unwrap_or_default() / 1_000_000_000);
                info!("   Value: {} ETH", ethers::utils::format_ether(tx.value));
                
                // Here you would analyze and potentially frontrun/backrun
            }
        }
        
        if tx_count % 100 == 0 {
            info!("📊 Processed {} transactions", tx_count);
        }
    }
    
    Ok(())
}

fn is_dex_trade(tx: &Transaction) -> bool {
    let uniswap_router: Address = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse().unwrap();
    let sushiswap_router: Address = "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse().unwrap();
    
    tx.to == Some(uniswap_router) || tx.to == Some(sushiswap_router)
}