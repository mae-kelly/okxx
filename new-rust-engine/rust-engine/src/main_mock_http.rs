use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use dotenv::dotenv;
use log::{info, warn};

mod monitor;
mod config;
mod arbitrage;
mod wallet;

use monitor::PriceMonitor;

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
    
    // Use HTTP provider instead of WebSocket
    let provider = Arc::new(
        Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?
    );
    
    info!("👀 Monitoring real Arbitrum mainnet prices...");
    info!("⏳ This may take a few minutes to find opportunities...\n");
    
    let factory_abi = ethers::abi::parse_abi(&[
        "function getPair(address,address) view returns (address)"
    ])?;
    
    let pair_abi = ethers::abi::parse_abi(&[
        "function getReserves() view returns (uint112,uint112,uint32)"
    ])?;
    
    let uniswap_factory = "0xf1D7CC64Fb4452F05c498126312eBE29f30Fbcf9".parse::<Address>()?;
    let sushiswap_factory = "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse::<Address>()?;
    
    let pairs = vec![
        ("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"), // WETH/USDC
        ("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", "0x912CE59144191C1204E64559FE8253a0e49E6548"), // WETH/ARB
    ];
    
    let mut interval = interval(Duration::from_secs(2)); // Check every 2 seconds
    let mut scan_count = 0;
    
    loop {
        interval.tick().await;
        scan_count += 1;
        
        // Get real gas price
        let gas_price = provider.get_gas_price().await?;
        let gas_cost = gas_price * U256::from(400000);
        
        // Check each pair
        for (token_a, token_b) in &pairs {
            let token_a = token_a.parse::<Address>()?;
            let token_b = token_b.parse::<Address>()?;
            
            // Get Uniswap pair
            let uni_factory_contract = Contract::new(
                uniswap_factory,
                factory_abi.clone(),
                provider.clone()
            );
            
            let uni_pair: Address = uni_factory_contract
                .method("getPair", (token_a, token_b))?
                .call().await?;
            
            if uni_pair == Address::zero() {
                continue;
            }
            
            // Get Sushiswap pair
            let sushi_factory_contract = Contract::new(
                sushiswap_factory,
                factory_abi.clone(),
                provider.clone()
            );
            
            let sushi_pair: Address = sushi_factory_contract
                .method("getPair", (token_a, token_b))?
                .call().await?;
            
            if sushi_pair == Address::zero() {
                continue;
            }
            
            // Get reserves from both
            let uni_contract = Contract::new(uni_pair, pair_abi.clone(), provider.clone());
            let sushi_contract = Contract::new(sushi_pair, pair_abi.clone(), provider.clone());
            
            let uni_reserves: (U256, U256, U256) = uni_contract
                .method("getReserves", ())?
                .call().await?;
                
            let sushi_reserves: (U256, U256, U256) = sushi_contract
                .method("getReserves", ())?
                .call().await?;
            
            // Calculate prices
            let uni_price = if uni_reserves.1 > U256::zero() {
                (uni_reserves.0 * U256::from(10u64.pow(18))) / uni_reserves.1
            } else {
                continue;
            };
            
            let sushi_price = if sushi_reserves.1 > U256::zero() {
                (sushi_reserves.0 * U256::from(10u64.pow(18))) / sushi_reserves.1
            } else {
                continue;
            };
            
            // Calculate price difference
            let price_diff_percent = if uni_price > sushi_price {
                ((uni_price - sushi_price) * U256::from(1000)) / sushi_price
            } else {
                ((sushi_price - uni_price) * U256::from(1000)) / uni_price
            };
            
            // If difference > 0.3% (3 per thousand)
            if price_diff_percent > U256::from(3) {
                total_trades += 1;
                
                let optimal_amount = U256::from(10u64.pow(17)); // 0.1 ETH
                let potential_profit = (optimal_amount * price_diff_percent) / U256::from(1000);
                
                // Account for fees (0.6% total)
                let fees = optimal_amount * U256::from(6) / U256::from(1000);
                let profit_after_fees = if potential_profit > fees {
                    potential_profit - fees
                } else {
                    U256::zero()
                };
                
                let profit_after_gas = if profit_after_fees > gas_cost {
                    profit_after_fees - gas_cost
                } else {
                    U256::zero()
                };
                
                if profit_after_gas > U256::zero() {
                    profitable_trades += 1;
                    total_profit += profit_after_gas;
                    mock_balance += profit_after_gas;
                    
                    info!("💎 OPPORTUNITY FOUND!");
                    info!("   Pair: WETH/USDC");
                    info!("   Price diff: {:.2}%", price_diff_percent.as_u128() as f64 / 10.0);
                    info!("   Route: {} → {}", 
                        if uni_price < sushi_price { "Uniswap" } else { "Sushiswap" },
                        if uni_price < sushi_price { "Sushiswap" } else { "Uniswap" });
                    info!("   Profit: {} ETH (${:.2})", 
                        ethers::utils::format_ether(profit_after_gas),
                        ethers::utils::format_ether(profit_after_gas).parse::<f64>().unwrap_or(0.0) * 2000.0);
                    
                    info!("\n📊 STATISTICS:");
                    info!("   Scans: {} | Opportunities: {} | Success rate: {:.1}%", 
                        scan_count, total_trades,
                        if total_trades > 0 { (profitable_trades as f64 / total_trades as f64) * 100.0 } else { 0.0 });
                    info!("   Total profit: {} ETH", ethers::utils::format_ether(total_profit));
                    info!("   Mock balance: {} ETH\n", ethers::utils::format_ether(mock_balance));
                } else {
                    total_gas_spent += gas_cost;
                    mock_balance -= gas_cost;
                    
                    warn!("   ⚠️ Opportunity found but not profitable after gas!");
                }
            }
        }
        
        // Show we're still scanning every 20 iterations
        if scan_count % 20 == 0 {
            info!("👀 Scan #{}: Still monitoring... (Gas: {} Gwei)", 
                scan_count, gas_price / 1_000_000_000);
        }
    }
}