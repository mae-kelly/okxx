use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use dotenv::dotenv;
use log::{info, warn};

mod wallet;
mod arbitrage;
mod monitor;
mod config;

use monitor::PriceMonitor;
use config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    env_logger::init();
    
    info!("🧪 REALISTIC MOCK MODE - Real data, fake execution");
    info!("📊 Using live prices, real gas costs, actual fees");
    info!("💰 Starting with simulated 0.1 ETH balance");
    
    let mut mock_balance = ethers::utils::parse_ether("0.1")?;
    let mut total_trades = 0;
    let mut profitable_trades = 0;
    let mut total_profit = U256::zero();
    let mut total_gas_spent = U256::zero();
    
    // Connect to real Arbitrum for live data
    let config = Config::from_env()?;
    let provider = Arc::new(
        Provider::<Ws>::connect(&config.ws_url).await?
    );
    
    let monitor = PriceMonitor::new(provider.clone(), config.clone());
    
    info!("👀 Monitoring real Arbitrum mainnet prices...\n");
    
    let mut interval = interval(Duration::from_millis(500));
    
    loop {
        interval.tick().await;
        
        // Get real gas price
        let gas_price = provider.get_gas_price().await?;
        let gas_cost = gas_price * U256::from(400000); // ~400k gas for arb
        
        // Find real arbitrage opportunities
        match monitor.find_arbitrage_opportunity().await {
            Ok(Some(opp)) => {
                total_trades += 1;
                
                // Calculate real costs
                let gas_cost_eth = ethers::utils::format_ether(gas_cost);
                let profit_eth = ethers::utils::format_ether(opp.profit_after_gas);
                
                info!("💎 REAL OPPORTUNITY FOUND!");
                info!("   Pair: {:?} <-> {:?}", opp.token_a, opp.token_b);
                info!("   Route: {} → {}", opp.buy_from_dex, opp.sell_to_dex);
                info!("   Amount: {} ETH", ethers::utils::format_ether(opp.optimal_amount));
                info!("   Gas price: {} Gwei", gas_price / 1_000_000_000);
                info!("   Gas cost: {} ETH (${})", gas_cost_eth, 
                    gas_cost_eth.parse::<f64>().unwrap_or(0.0) * 2000.0);
                
                // Include DEX fees (0.3% each swap = 0.6% total)
                let dex_fees = opp.optimal_amount * 6 / 1000;
                let dex_fees_eth = ethers::utils::format_ether(dex_fees);
                info!("   DEX fees (0.6%): {} ETH", dex_fees_eth);
                
                // Check if we have enough balance
                if opp.optimal_amount + gas_cost > mock_balance {
                    warn!("   ❌ MOCK: Insufficient balance!");
                    warn!("      Need: {} ETH", 
                        ethers::utils::format_ether(opp.optimal_amount + gas_cost));
                    warn!("      Have: {} ETH", 
                        ethers::utils::format_ether(mock_balance));
                    continue;
                }
                
                // Simulate slippage (0.1-0.5% random) - fix the type issue
                let random_factor = U256::from(rand::random::<u8>());
                let slippage = opp.profit_after_gas * random_factor / U256::from(2000);
                let final_profit = if opp.profit_after_gas > slippage {
                    opp.profit_after_gas - slippage
                } else {
                    U256::zero()
                };
                
                if final_profit > U256::zero() {
                    profitable_trades += 1;
                    total_profit += final_profit;
                    mock_balance += final_profit;
                    mock_balance -= gas_cost;
                    total_gas_spent += gas_cost;
                    
                    info!("   ✅ MOCK EXECUTION:");
                    info!("      Gross profit: {} ETH", profit_eth);
                    info!("      After slippage: {} ETH", 
                        ethers::utils::format_ether(final_profit));
                    info!("      Net profit: ${:.2}", 
                        ethers::utils::format_ether(final_profit).parse::<f64>().unwrap_or(0.0) * 2000.0);
                    
                } else {
                    mock_balance -= gas_cost;
                    total_gas_spent += gas_cost;
                    
                    warn!("   ⚠️ MOCK: Would lose money after slippage!");
                }
                
                info!("\n📊 MOCK STATISTICS:");
                info!("   Total trades: {}", total_trades);
                info!("   Profitable: {} ({:.1}%)", 
                    profitable_trades, 
                    if total_trades > 0 { (profitable_trades as f64 / total_trades as f64) * 100.0 } else { 0.0 });
                info!("   Total profit: {} ETH", 
                    ethers::utils::format_ether(total_profit));
                info!("   Total gas spent: {} ETH", 
                    ethers::utils::format_ether(total_gas_spent));
                info!("   Current balance: {} ETH", 
                    ethers::utils::format_ether(mock_balance));
                
                let initial_balance = ethers::utils::parse_ether("0.1")?;
                let roi = if initial_balance > U256::zero() {
                    ((mock_balance.as_u128() as f64 / initial_balance.as_u128() as f64) - 1.0) * 100.0
                } else {
                    0.0
                };
                info!("   ROI: {:.2}%\n", roi);
                
                // Realistic delay for block confirmation
                tokio::time::sleep(Duration::from_secs(2)).await;
            }
            Ok(None) => {
                // No opportunity - this is normal 99% of the time
            }
            Err(e) => {
                warn!("Monitor error: {}", e);
            }
        }
    }
}