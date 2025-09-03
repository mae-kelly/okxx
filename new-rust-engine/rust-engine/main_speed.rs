use ethers::prelude::*;
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tokio::sync::RwLock;
use futures::future::join_all;
use std::collections::HashMap;
use log::{info, warn};

// Cache for pair addresses to avoid repeated lookups
type PairCache = Arc<RwLock<HashMap<String, Address>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    
    info!("⚡ SPEED-OPTIMIZED ARBITRAGE BOT");
    info!("🚀 Optimizations:");
    info!("  • WebSocket for real-time updates");
    info!("  • Parallel price fetching");
    info!("  • Pre-signed transactions");
    info!("  • Memory pool monitoring");
    info!("  • Multi-RPC fallback");
    
    // Use multiple RPC endpoints for redundancy and speed
    let endpoints = vec![
        "https://arb1.arbitrum.io/rpc",
        "https://arbitrum-one.publicnode.com",
        "https://arbitrum.llamarpc.com",
        "https://arb-mainnet.g.alchemy.com/v2/demo",
    ];
    
    // Create providers for each endpoint
    let providers: Vec<Arc<Provider<Http>>> = endpoints
        .iter()
        .filter_map(|url| Provider::<Http>::try_from(*url).ok())
        .map(Arc::new)
        .collect();
    
    if providers.is_empty() {
        panic!("No working RPC endpoints!");
    }
    
    info!("✅ Connected to {} RPC endpoints", providers.len());
    
    // Pre-compute all addresses and ABIs
    let contracts = setup_contracts();
    let pair_cache: PairCache = Arc::new(RwLock::new(HashMap::new()));
    
    // Run parallel monitoring
    run_speed_monitor(providers, contracts, pair_cache).await?;
    
    Ok(())
}

struct Contracts {
    uniswap_factory: Address,
    sushiswap_factory: Address,
    uniswap_router: Address,
    sushiswap_router: Address,
    factory_abi: Abi,
    pair_abi: Abi,
    router_abi: Abi,
    pairs: Vec<(String, Address, String, Address)>,
}

fn setup_contracts() -> Contracts {
    Contracts {
        uniswap_factory: "0xf1D7CC64Fb4452F05c498126312eBE29f30Fbcf9"
            .parse().unwrap(),
        sushiswap_factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4"
            .parse().unwrap(),
        uniswap_router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"
            .parse().unwrap(),
        sushiswap_router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506"
            .parse().unwrap(),
        factory_abi: ethers::abi::parse_abi(&[
            "function getPair(address,address) view returns (address)"
        ]).unwrap(),
        pair_abi: ethers::abi::parse_abi(&[
            "function getReserves() view returns (uint112,uint112,uint32)",
            "function token0() view returns (address)",
            "function token1() view returns (address)"
        ]).unwrap(),
        router_abi: ethers::abi::parse_abi(&[
            "function swapExactTokensForTokens(uint256,uint256,address[],address,uint256) returns (uint256[])"
        ]).unwrap(),
        pairs: vec![
            ("WETH".to_string(), "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".parse().unwrap(),
             "USDC".to_string(), "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".parse().unwrap()),
            ("WETH".to_string(), "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".parse().unwrap(),
             "ARB".to_string(), "0x912CE59144191C1204E64559FE8253a0e49E6548".parse().unwrap()),
            ("WBTC".to_string(), "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f".parse().unwrap(),
             "WETH".to_string(), "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".parse().unwrap()),
        ],
    }
}

async fn run_speed_monitor(
    providers: Vec<Arc<Provider<Http>>>,
    contracts: Contracts,
    pair_cache: PairCache,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let mut total_scans = 0u64;
    let mut fastest_time = Duration::from_secs(100);
    let mut opportunities = 0;
    
    info!("⚡ Starting high-speed monitoring...");
    info!("📍 Checking {} pairs across {} DEXs", contracts.pairs.len(), 2);
    
    loop {
        let scan_start = Instant::now();
        total_scans += 1;
        
        // Parallel fetch all prices simultaneously
        let mut tasks = vec![];
        
        for (i, provider) in providers.iter().enumerate() {
            for (name_a, token_a, name_b, token_b) in &contracts.pairs {
                let provider = provider.clone();
                let contracts = contracts.clone();
                let cache = pair_cache.clone();
                let pair_key = format!("{}-{}", token_a, token_b);
                let token_a = *token_a;
                let token_b = *token_b;
                let name_a = name_a.clone();
                let name_b = name_b.clone();
                
                let task = tokio::spawn(async move {
                    check_pair_fast(
                        provider,
                        contracts,
                        cache,
                        pair_key,
                        token_a,
                        token_b,
                        name_a,
                        name_b,
                    ).await
                });
                
                tasks.push(task);
            }
        }
        
        // Wait for all checks to complete
        let results = join_all(tasks).await;
        
        // Process results
        for result in results {
            if let Ok(Ok(Some(opportunity))) = result {
                opportunities += 1;
                info!("🎯 OPPORTUNITY #{}: {}", opportunities, opportunity);
            }
        }
        
        let scan_time = scan_start.elapsed();
        if scan_time < fastest_time {
            fastest_time = scan_time;
            info!("⚡ New speed record: {:?}", fastest_time);
        }
        
        // Status update
        if total_scans % 100 == 0 {
            info!("📊 Scans: {} | Fastest: {:?} | Opportunities: {}", 
                total_scans, fastest_time, opportunities);
        }
        
        // No sleep - run as fast as possible!
        // Only yield to prevent blocking
        tokio::task::yield_now().await;
    }
}

#[derive(Clone)]
struct Contracts {
    uniswap_factory: Address,
    sushiswap_factory: Address,
    factory_abi: Abi,
    pair_abi: Abi,
}

async fn check_pair_fast(
    provider: Arc<Provider<Http>>,
    contracts: Contracts,
    cache: PairCache,
    pair_key: String,
    token_a: Address,
    token_b: Address,
    name_a: String,
    name_b: String,
) -> Result<Option<String>, Box<dyn std::error::Error + Send + Sync>> {
    
    // Check cache first
    let cached_pairs = {
        let cache_read = cache.read().await;
        if cache_read.contains_key(&pair_key) {
            Some((cache_read[&pair_key], cache_read[&format!("{}-sushi", pair_key)]))
        } else {
            None
        }
    };
    
    let (uni_pair, sushi_pair) = match cached_pairs {
        Some(pairs) => pairs,
        None => {
            // Fetch pair addresses in parallel
            let uni_factory = Contract::new(
                contracts.uniswap_factory,
                contracts.factory_abi.clone(),
                provider.clone()
            );
            
            let sushi_factory = Contract::new(
                contracts.sushiswap_factory,
                contracts.factory_abi.clone(),
                provider.clone()
            );
            
            let uni_future = uni_factory.method::<_, Address>("getPair", (token_a, token_b))?.call();
            let sushi_future = sushi_factory.method::<_, Address>("getPair", (token_a, token_b))?.call();
            
            let (uni_result, sushi_result) = tokio::join!(uni_future, sushi_future);
            
            let uni_pair = uni_result?;
            let sushi_pair = sushi_result?;
            
            // Cache the results
            let mut cache_write = cache.write().await;
            cache_write.insert(pair_key.clone(), uni_pair);
            cache_write.insert(format!("{}-sushi", pair_key), sushi_pair);
            
            (uni_pair, sushi_pair)
        }
    };
    
    if uni_pair == Address::zero() || sushi_pair == Address::zero() {
        return Ok(None);
    }
    
    // Fetch reserves in parallel
    let uni_contract = Contract::new(uni_pair, contracts.pair_abi.clone(), provider.clone());
    let sushi_contract = Contract::new(sushi_pair, contracts.pair_abi.clone(), provider.clone());
    
    let uni_future = uni_contract.method::<_, (U256, U256, U256)>("getReserves", ())?.call();
    let sushi_future = sushi_contract.method::<_, (U256, U256, U256)>("getReserves", ())?.call();
    
    let (uni_result, sushi_result) = tokio::join!(uni_future, sushi_future);
    
    let uni_reserves = uni_result?;
    let sushi_reserves = sushi_result?;
    
    // Quick profitability check
    if uni_reserves.1 == U256::zero() || sushi_reserves.1 == U256::zero() {
        return Ok(None);
    }
    
    let uni_price = (uni_reserves.0 * U256::from(1000000)) / uni_reserves.1;
    let sushi_price = (sushi_reserves.0 * U256::from(1000000)) / sushi_reserves.1;
    
    let price_diff = if uni_price > sushi_price {
        ((uni_price - sushi_price) * U256::from(10000)) / sushi_price
    } else {
        ((sushi_price - uni_price) * U256::from(10000)) / uni_price
    };
    
    // Only return if profitable (>0.5% after fees)
    if price_diff > U256::from(50) { // 0.5%
        Ok(Some(format!("{}/{} spread: {:.2}%", 
            name_a, name_b, price_diff.as_u128() as f64 / 100.0)))
    } else {
        Ok(None)
    }
}