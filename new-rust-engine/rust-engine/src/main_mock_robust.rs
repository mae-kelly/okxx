use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{Duration, interval, sleep};
use log::{info, warn, error};
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("🧪 ARBITRAGE BOT MOCK - Real Data Analysis");
    info!("=" .repeat(50));
    info!("📊 Using: Live Arbitrum prices");
    info!("⛽ Gas: Real-time gas costs");
    info!("💰 Capital: Simulated 0.1 ETH");
    info!("=" .repeat(50));
    
    let mut mock_balance = ethers::utils::parse_ether("0.1")?;
    let mut total_scans = 0;
    let mut opportunities_found = 0;
    let mut profitable_trades = 0;
    let mut total_profit = U256::zero();
    
    // Pairs to monitor
    let pairs = vec![
        ("WETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", "USDC", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
        ("WETH", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", "ARB", "0x912CE59144191C1204E64559FE8253a0e49E6548"),
    ];
    
    info!("\n📈 REAL-TIME MARKET DATA:");
    info!("Monitoring {} pairs on Uniswap vs Sushiswap", pairs.len());
    info!("Checking every 2 seconds for price differences\n");
    
    // Add some realistic context
    info!("⚠️  REALITY CHECK:");
    info!("• MEV bots with direct mempool access dominate");
    info!("• They execute in <100ms, we're checking every 2000ms");
    info!("• Most opportunities are taken before we see them");
    info!("• This shows what WOULD be profitable if we were faster\n");
    
    let mut retry_count = 0;
    let max_retries = 3;
    
    loop {
        match run_scanner(&pairs, &mut total_scans, &mut opportunities_found, 
                         &mut profitable_trades, &mut mock_balance, &mut total_profit).await {
            Ok(_) => {
                retry_count = 0;
            }
            Err(e) => {
                error!("Connection error: {}", e);
                retry_count += 1;
                
                if retry_count >= max_retries {
                    error!("Max retries reached. Showing final stats...");
                    break;
                }
                
                warn!("Retrying in 5 seconds... (attempt {}/{})", retry_count, max_retries);
                sleep(Duration::from_secs(5)).await;
            }
        }
    }
    
    // Final statistics
    show_final_stats(total_scans, opportunities_found, profitable_trades, total_profit, mock_balance);
    
    Ok(())
}

async fn run_scanner(
    pairs: &[(& str, &str, &str, &str)],
    total_scans: &mut u32,
    opportunities_found: &mut u32,
    profitable_trades: &mut u32,
    mock_balance: &mut U256,
    total_profit: &mut U256,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let provider = Arc::new(
        Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?
    );
    
    let factory_abi = ethers::abi::parse_abi(&[
        "function getPair(address,address) view returns (address)"
    ])?;
    
    let pair_abi = ethers::abi::parse_abi(&[
        "function getReserves() view returns (uint112,uint112,uint32)"
    ])?;
    
    let uniswap_factory = "0xf1D7CC64Fb4452F05c498126312eBE29f30Fbcf9".parse::<Address>()?;
    let sushiswap_factory = "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse::<Address>()?;
    
    let mut interval = interval(Duration::from_secs(2));
    
    for _ in 0..50 { // Run 50 scans then reconnect
        interval.tick().await;
        *total_scans += 1;
        
        let gas_price = provider.get_gas_price().await?;
        let gas_cost = gas_price * U256::from(400000);
        
        for (name_a, token_a, name_b, token_b) in pairs {
            let token_a = token_a.parse::<Address>()?;
            let token_b = token_b.parse::<Address>()?;
            
            // Get pairs
            let uni_factory_contract = Contract::new(
                uniswap_factory,
                factory_abi.clone(),
                provider.clone()
            );
            
            let sushi_factory_contract = Contract::new(
                sushiswap_factory,
                factory_abi.clone(),
                provider.clone()
            );
            
            let uni_pair: Address = uni_factory_contract
                .method("getPair", (token_a, token_b))?.call().await?;
            
            let sushi_pair: Address = sushi_factory_contract
                .method("getPair", (token_a, token_b))?.call().await?;
            
            if uni_pair == Address::zero() || sushi_pair == Address::zero() {
                continue;
            }
            
            // Get reserves
            let uni_contract = Contract::new(uni_pair, pair_abi.clone(), provider.clone());
            let sushi_contract = Contract::new(sushi_pair, pair_abi.clone(), provider.clone());
            
            let uni_reserves: (U256, U256, U256) = uni_contract
                .method("getReserves", ())?.call().await?;
            let sushi_reserves: (U256, U256, U256) = sushi_contract
                .method("getReserves", ())?.call().await?;
            
            // Calculate price difference
            if uni_reserves.1 == U256::zero() || sushi_reserves.1 == U256::zero() {
                continue;
            }
            
            let uni_price = (uni_reserves.0 * U256::from(10u64.pow(18))) / uni_reserves.1;
            let sushi_price = (sushi_reserves.0 * U256::from(10u64.pow(18))) / sushi_reserves.1;
            
            let price_diff = if uni_price > sushi_price {
                ((uni_price - sushi_price) * U256::from(10000)) / sushi_price
            } else {
                ((sushi_price - uni_price) * U256::from(10000)) / uni_price
            };
            
            // Show current spread (even if not profitable)
            if *total_scans % 10 == 0 && price_diff > U256::zero() {
                info!("📍 Scan #{}: {}/{} spread: {:.3}%", 
                    total_scans, name_a, name_b,
                    price_diff.as_u128() as f64 / 100.0);
            }
            
            // If spread > 0.3% (30 basis points)
            if price_diff > U256::from(30) {
                *opportunities_found += 1;
                
                let amount = U256::from(10u64.pow(17)); // 0.1 ETH
                let fees = amount * U256::from(60) / U256::from(10000); // 0.6%
                let potential = (amount * price_diff) / U256::from(10000);
                
                if potential > fees + gas_cost {
                    *profitable_trades += 1;
                    let profit = potential - fees - gas_cost;
                    *total_profit += profit;
                    *mock_balance += profit;
                    
                    info!("\n🎯 PROFITABLE OPPORTUNITY!");
                    info!("   Pair: {}/{}", name_a, name_b);
                    info!("   Spread: {:.2}%", price_diff.as_u128() as f64 / 100.0);
                    info!("   Route: {} → {}", 
                        if uni_price < sushi_price { "Uniswap" } else { "Sushiswap" },
                        if uni_price < sushi_price { "Sushiswap" } else { "Uniswap" });
                    info!("   Profit: {} ETH (${:.2})",
                        ethers::utils::format_ether(profit),
                        ethers::utils::format_ether(profit).parse::<f64>()? * 2000.0);
                    info!("   Status: ✅ MOCK EXECUTED\n");
                } else {
                    warn!("❌ Opportunity found but not profitable after gas!");
                    warn!("   Spread: {:.2}% | Gas cost too high", 
                        price_diff.as_u128() as f64 / 100.0);
                }
            }
        }
        
        // Status update
        if *total_scans % 20 == 0 {
            info!("📊 Status: {} scans | {} opportunities | {} profitable | Balance: {} ETH",
                total_scans, opportunities_found, profitable_trades,
                ethers::utils::format_ether(*mock_balance));
        }
    }
    
    Ok(())
}

fn show_final_stats(scans: u32, opportunities: u32, profitable: u32, profit: U256, balance: U256) {
    info!("\n{'='} FINAL STATISTICS {'='}");
    info!("Total scans: {}", scans);
    info!("Opportunities detected: {}", opportunities);
    info!("Profitable trades: {}", profitable);
    info!("Success rate: {:.1}%", 
        if opportunities > 0 { (profitable as f64 / opportunities as f64) * 100.0 } else { 0.0 });
    info!("Total profit: {} ETH", ethers::utils::format_ether(profit));
    info!("Final balance: {} ETH", ethers::utils::format_ether(balance));
    
    let initial = ethers::utils::parse_ether("0.1").unwrap();
    let roi = if initial > U256::zero() {
        ((balance.as_u128() as f64 / initial.as_u128() as f64) - 1.0) * 100.0
    } else { 0.0 };
    info!("ROI: {:.2}%", roi);
    
    info!("\n💡 INSIGHTS:");
    if profitable == 0 {
        info!("• No profitable opportunities found - this is normal!");
        info!("• Professional MEV bots capture most arbitrage");
        info!("• You'd need <10ms latency to compete");
    } else {
        info!("• Found {} opportunities that WOULD be profitable", profitable);
        info!("• In reality, MEV bots would execute these first");
        info!("• Consider focusing on less competitive strategies");
    }
}