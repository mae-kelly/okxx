use ethers::prelude::*;
use ethers::utils::format_units;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use dotenv::dotenv;
use log::{info, warn, error};

mod wallet;
mod arbitrage;
mod monitor;
mod flashloan;
mod config;

use wallet::WalletManager;
use arbitrage::ArbitrageExecutor;
use monitor::PriceMonitor;
use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize environment
    dotenv().ok();
    env_logger::init();
    
    info!("🚀 Starting Arbitrage Bot v0.2.0");
    
    // Load configuration
    let config = Config::from_env()?;
    
    // Initialize wallet
    let wallet = WalletManager::new()?;
    info!("📱 Wallet address: {}", wallet.address());
    
    // Check wallet balance
    let balance = wallet.get_balance().await?;
    info!("💰 Wallet balance: {} ETH", format_units(balance, "ether")?);
    
    if balance < ethers::utils::parse_ether("0.01")? {
        error!("❌ Insufficient balance for gas! Need at least 0.01 ETH");
        return Ok(());
    }
    
    // Initialize components
    let provider = Arc::new(
        Provider::<Ws>::connect(&config.ws_url).await?
    );
    
    let monitor = PriceMonitor::new(provider.clone(), config.clone());
    let executor = ArbitrageExecutor::new(wallet, provider.clone(), config.clone());
    
    // Start monitoring
    info!("👀 Starting price monitoring...");
    
    let mut interval = interval(Duration::from_millis(100)); // Fast polling
    
    loop {
        interval.tick().await;
        
        match monitor.find_arbitrage_opportunity().await {
            Ok(Some(opportunity)) => {
                info!("💎 Found opportunity: {:?}", opportunity);
                
                // Only execute if profitable after gas
                if opportunity.profit_after_gas > config.min_profit_wei {
                    info!("🎯 Executing arbitrage...");
                    
                    match executor.execute_with_flashloan(opportunity).await {
                        Ok(tx_hash) => {
                            info!("✅ Success! Transaction: {:?}", tx_hash);
                        }
                        Err(e) => {
                            error!("❌ Execution failed: {}", e);
                        }
                    }
                }
            }
            Ok(None) => {
                // No opportunity found, continue monitoring
            }
            Err(e) => {
                warn!("⚠️ Monitor error: {}", e);
            }
        }
    }
}