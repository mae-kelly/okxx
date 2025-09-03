use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{Duration, interval};
use log::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("🌐 MULTI-CHAIN ARBITRAGE SCANNER");
    info!("Scanning less competitive chains for opportunities");
    
    // Start with Polygon - good balance of volume and low competition
    scan_polygon().await
}

async fn scan_polygon() -> Result<(), Box<dyn std::error::Error>> {
    info!("🟣 Starting Polygon scanner...");
    
    let provider = Arc::new(
        Provider::<Http>::try_from("https://polygon-rpc.com")?
    );
    
    let quickswap = "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32".parse::<Address>()?;
    let sushiswap = "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse::<Address>()?;
    
    let pairs = vec![
        ("WMATIC", "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270"),
        ("USDC", "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174"),
        ("USDT", "0xc2132D05D31c914a87C6611C10748AEb04B58e8F"),
    ];
    
    let factory_abi = ethers::abi::parse_abi(&[
        "function getPair(address,address) view returns (address)"
    ])?;
    
    let pair_abi = ethers::abi::parse_abi(&[
        "function getReserves() view returns (uint112,uint112,uint32)"
    ])?;
    
    let mut interval = interval(Duration::from_secs(2));
    let mut scans = 0;
    
    info!("Competition level: 🟢 LOW (5-10 bots vs 50+ on Arbitrum)");
    info!("Gas cost: $0.01 (vs $0.10+ on Arbitrum)");
    info!("Monitoring QuickSwap vs SushiSwap...\n");
    
    loop {
        interval.tick().await;
        scans += 1;
        
        for i in 0..pairs.len() {
            for j in i+1..pairs.len() {
                let (name_a, addr_a) = pairs[i];
                let (name_b, addr_b) = pairs[j];
                
                let token_a = addr_a.parse::<Address>()?;
                let token_b = addr_b.parse::<Address>()?;
                
                // Get QuickSwap pair
                let quick_factory = Contract::new(
                    quickswap,
                    factory_abi.clone(),
                    provider.clone()
                );
                
                let quick_pair: Address = quick_factory
                    .method("getPair", (token_a, token_b))?
                    .call().await?;
                
                if quick_pair == Address::zero() {
                    continue;
                }
                
                // Get Sushi pair
                let sushi_factory = Contract::new(
                    sushiswap,
                    factory_abi.clone(),
                    provider.clone()
                );
                
                let sushi_pair: Address = sushi_factory
                    .method("getPair", (token_a, token_b))?
                    .call().await?;
                
                if sushi_pair == Address::zero() {
                    continue;
                }
                
                // Get reserves
                let quick_contract = Contract::new(quick_pair, pair_abi.clone(), provider.clone());
                let sushi_contract = Contract::new(sushi_pair, pair_abi.clone(), provider.clone());
                
                let quick_reserves: (U256, U256, U256) = quick_contract
                    .method("getReserves", ())?
                    .call().await?;
                    
                let sushi_reserves: (U256, U256, U256) = sushi_contract
                    .method("getReserves", ())?
                    .call().await?;
                
                // Calculate price difference
                if quick_reserves.1 > U256::zero() && sushi_reserves.1 > U256::zero() {
                    let quick_price = (quick_reserves.0 * U256::from(1000000)) / quick_reserves.1;
                    let sushi_price = (sushi_reserves.0 * U256::from(1000000)) / sushi_reserves.1;
                    
                    let diff = if quick_price > sushi_price {
                        ((quick_price - sushi_price) * U256::from(10000)) / sushi_price
                    } else {
                        ((sushi_price - quick_price) * U256::from(10000)) / quick_price
                    };
                    
                    if diff > U256::from(30) { // 0.3%
                        info!("💎 OPPORTUNITY on Polygon!");
                        info!("   Pair: {}/{}", name_a, name_b);
                        info!("   Spread: {:.2}%", diff.as_u128() as f64 / 100.0);
                        info!("   Route: {} → {}", 
                            if quick_price < sushi_price { "QuickSwap" } else { "SushiSwap" },
                            if quick_price < sushi_price { "SushiSwap" } else { "QuickSwap" });
                    }
                }
            }
        }
        
        if scans % 30 == 0 {
            info!("📊 Scans: {} | Gas: ~$0.01 | Still monitoring Polygon...", scans);
        }
    }
}