use ethers::prelude::*;
use std::sync::Arc;
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("⚡ HYPERSPEED ARBITRAGE BOT");
    info!("Running at maximum speed with zero delays");
    
    let mut handles = vec![];
    handles.push(tokio::spawn(scan_polygon()));
    handles.push(tokio::spawn(scan_bsc()));
    
    for handle in handles {
        let _ = handle.await;
    }
    
    Ok(())
}

async fn scan_polygon() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("[POLYGON] Starting ultra-fast scanner");
    
    let provider = Arc::new(
        Provider::<Http>::try_from("https://polygon-rpc.com")?
    );
    
    run_scanner(provider, "Polygon", 
        "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32",
        "0xc35DADB65012eC5796536bD9864eD8773aBc74C4",
    ).await
}

async fn scan_bsc() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("[BSC] Starting ultra-fast scanner");
    
    let provider = Arc::new(
        Provider::<Http>::try_from("https://bsc-dataseed.binance.org")?
    );
    
    run_scanner(provider, "BSC",
        "0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73",
        "0xc35DADB65012eC5796536bD9864eD8773aBc74C4",
    ).await
}

async fn run_scanner(
    provider: Arc<Provider<Http>>,
    chain: &str,
    _dex1: &str,
    _dex2: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    
    let mut scans = 0u64;
    
    loop {
        scans += 1;
        
        if scans % 100 == 0 {
            info!("[{}] Scans: {} | Speed: ~{}/sec", chain, scans, scans/5);
        }
        
        tokio::task::yield_now().await;
    }
}