use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{Duration, Instant};

const UNISWAP_FACTORY: &str = "0xf1D7CC64Fb4452F05c498126312eBE29f30Fbcf9";
const SUSHISWAP_FACTORY: &str = "0xc35DADB65012eC5796536bD9864eD8773aBc74C4";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ULTRA-FAST ARBITRAGE SCANNER\n");
    
    let provider = Arc::new(Provider::<Http>::try_from(
        "https://arb-mainnet.g.alchemy.com/v2/alcht_oZ7wU7JpIoZejlOWUcMFOpNsIlLDsX"
    )?);
    
    let factory_abi = ethers::abi::parse_abi(&[
        "function getPair(address,address) view returns (address)"
    ])?;
    
    let pair_abi = ethers::abi::parse_abi(&[
        "function getReserves() view returns (uint112,uint112,uint32)"
    ])?;
    
    // Known high-activity pairs for speed
    let pairs = vec![
        ("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", "0xaf88d065e77c8cC2239327C5EDb3A432268e5831"), // WETH/USDC
        ("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", "0x912CE59144191C1204E64559FE8253a0e49E6548"), // WETH/ARB
        ("0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"), // WBTC/WETH
    ];
    
    loop {
        let start = Instant::now();
        let gas_price = provider.get_gas_price().await?;
        let gas_cost = (gas_price.as_u64() as f64 * 350_000.0 * 2000.0) / 1e18;
        
        // Parallel requests for speed
        let mut handles = vec![];
        
        for (token0, token1) in &pairs {
            let provider = provider.clone();
            let factory_abi = factory_abi.clone();
            let pair_abi = pair_abi.clone();
            let t0 = token0.parse::<Address>()?;
            let t1 = token1.parse::<Address>()?;
            
            let handle = tokio::spawn(async move {
                let uni_factory = Contract::new(
                    UNISWAP_FACTORY.parse::<Address>().unwrap(),
                    factory_abi.clone(),
                    provider.clone()
                );
                
                let sushi_factory = Contract::new(
                    SUSHISWAP_FACTORY.parse::<Address>().unwrap(),
                    factory_abi,
                    provider.clone()
                );
                
                // Get both pairs in parallel
                let uni_pair_future = uni_factory.method::<_, Address>("getPair", (t0, t1)).unwrap().call();
                let sushi_pair_future = sushi_factory.method::<_, Address>("getPair", (t0, t1)).unwrap().call();
                
                let (uni_addr, sushi_addr) = tokio::join!(uni_pair_future, sushi_pair_future);
                
                if let (Ok(uni_addr), Ok(sushi_addr)) = (uni_addr, sushi_addr) {
                    if uni_addr != Address::zero() && sushi_addr != Address::zero() {
                        let uni_pair = Contract::new(uni_addr, pair_abi.clone(), provider.clone());
                        let sushi_pair = Contract::new(sushi_addr, pair_abi, provider.clone());
                        
                        // Get reserves in parallel
                        let uni_res_future = uni_pair.method::<_, (U256, U256, U256)>("getReserves", ()).unwrap().call();
                        let sushi_res_future = sushi_pair.method::<_, (U256, U256, U256)>("getReserves", ()).unwrap().call();
                        
                        let (uni_res, sushi_res) = tokio::join!(uni_res_future, sushi_res_future);
                        
                        if let (Ok(ur), Ok(sr)) = (uni_res, sushi_res) {
                            let uni_price = ur.0.as_u128() as f64 / ur.1.as_u128().max(1) as f64;
                            let sushi_price = sr.0.as_u128() as f64 / sr.1.as_u128().max(1) as f64;
                            let spread = ((uni_price - sushi_price).abs() / uni_price.min(sushi_price)) * 100.0;
                            return Some(spread);
                        }
                    }
                }
                None
            });
            
            handles.push(handle);
        }
        
        // Collect results
        let results = futures::future::join_all(handles).await;
        
        println!("Scan time: {}ms | Gas: ${:.4}", start.elapsed().as_millis(), gas_cost);
        
        for (i, result) in results.iter().enumerate() {
            if let Ok(Some(spread)) = result {
                let profit = (100_000.0 * spread / 100.0) - gas_cost;
                if profit > 0.0 {
                    println!("  Pair {} | Spread: {:.3}% | $100k loan → ${:.2} profit", i, spread, profit);
                }
            }
        }
        
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}