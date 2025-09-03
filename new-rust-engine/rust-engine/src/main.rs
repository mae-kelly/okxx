// rust-engine/src/main.rs
use ethers::prelude::*;
use std::sync::Arc;
use std::env;
use tokio::time::{Duration, interval};
use dashmap::DashMap;
use parking_lot::RwLock;
use anyhow::Result;
use tracing::{info, warn, error};
use dotenv::dotenv;

pub mod config;
pub mod scanner;
pub mod flash_loan;
pub mod mempool;
pub mod contracts;
pub mod advanced_scanner;
pub mod multi_rpc;

use config::{ChainConfig, DexConfig};
use advanced_scanner::AdvancedScanner;
use multi_rpc::MultiRpcProvider;
use flash_loan::FlashLoanExecutor;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt::init();
    
    info!("🚀 ULTRA-FAST L2 ARBITRAGE SCANNER v3.0");
    info!("📊 Strategy: Full pair discovery + Smart caching + Multi-RPC rotation");
    
    // Get multiple RPC URLs for each chain
    let arbitrum_rpcs = get_rpc_list("ARBITRUM");
    let optimism_rpcs = get_rpc_list("OPTIMISM");
    let base_rpcs = get_rpc_list("BASE");
    // let _polygon_zkevm_rpcs = get_rpc_list("POLYGON_ZKEVM");
    // let _zksync_rpcs = get_rpc_list("ZKSYNC");
    
    let chains = vec![
        ChainInfo {
            config: ChainConfig {
                name: "Arbitrum".to_string(),
                rpc: arbitrum_rpcs[0].clone(),
                chain_id: 42161,
                flash_loan_providers: vec![
                    "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?,
                    "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse()?,
                ],
                dexes: vec![
                    DexConfig {
                        name: "Uniswap V3".to_string(),
                        factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse()?,
                        router: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "Sushiswap".to_string(),
                        factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse()?,
                        router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "Camelot".to_string(),
                        factory: "0x6EcCab422D763aC031210895C81787E87B43A652".parse()?,
                        router: "0xc873fEcbd354f5A56E00E710B90EF4201db2448d".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "TraderJoe".to_string(),
                        factory: "0xaE4EC9901c3076D0DdBe76A520F9E90a6227aCB7".parse()?,
                        router: "0xb4315e873dBcf96Ffd0acd8EA43f689D8c20fB30".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "Zyberswap".to_string(),
                        factory: "0xaC2ee06A14c52570Ef3B9812Ed240BCe359772e7".parse()?,
                        router: "0x16e71B13fE6079B4312063F7E81F76d165Ad32Ad".parse()?,
                        fee_bps: 30,
                    },
                ],
            },
            rpc_urls: arbitrum_rpcs,
            ws_url: env::var("ARBITRUM_WS").ok(),
        },
        ChainInfo {
            config: ChainConfig {
                name: "Optimism".to_string(),
                rpc: optimism_rpcs[0].clone(),
                chain_id: 10,
                flash_loan_providers: vec![
                    "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse()?,
                ],
                dexes: vec![
                    DexConfig {
                        name: "Velodrome".to_string(),
                        factory: "0x25CbdDb98b35ab1FF77413456B31EC81A6B6B746".parse()?,
                        router: "0xa062aE8A9c5e11aaA026fc2670B0D65cCc8B2858".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "Uniswap V3".to_string(),
                        factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse()?,
                        router: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "Beethoven X".to_string(),
                        factory: "0xB4C0c9bb7A82c5e0F14a62bA2A0C7ec8c5cD1267".parse()?,
                        router: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?,
                        fee_bps: 30,
                    },
                ],
            },
            rpc_urls: optimism_rpcs,
            ws_url: env::var("OPTIMISM_WS").ok(),
        },
        ChainInfo {
            config: ChainConfig {
                name: "Base".to_string(),
                rpc: base_rpcs[0].clone(),
                chain_id: 8453,
                flash_loan_providers: vec![
                    "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?,
                ],
                dexes: vec![
                    DexConfig {
                        name: "BaseSwap".to_string(),
                        factory: "0xFDa619b6d20975be80A10332cD39b9a4b0FAa8BB".parse()?,
                        router: "0x327Df1E6de05895d2ab08513aaDD9313Fe505d86".parse()?,
                        fee_bps: 25,
                    },
                    DexConfig {
                        name: "Aerodrome".to_string(),
                        factory: "0x420DD381b31aEf6683db6B902084cB0FFECe40Da".parse()?,
                        router: "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43".parse()?,
                        fee_bps: 30,
                    },
                    DexConfig {
                        name: "SushiSwap V3".to_string(),
                        factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse()?,
                        router: "0xFB7eF66a7e61224DD6FcD0D7d9C3be5C8B049b9f".parse()?,
                        fee_bps: 30,
                    },
                ],
            },
            rpc_urls: base_rpcs,
            ws_url: env::var("BASE_WS").ok(),
        },
    ];
    
    let mut handles = vec![];
    
    for chain_info in chains {
        let handle = tokio::spawn(async move {
            if let Err(e) = run_advanced_scanner(chain_info).await {
                error!("Advanced scanner error: {}", e);
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        let _ = handle.await;
    }
    
    Ok(())
}

struct ChainInfo {
    config: ChainConfig,
    rpc_urls: Vec<String>,
    ws_url: Option<String>,
}

async fn run_advanced_scanner(chain_info: ChainInfo) -> Result<()> {
    info!("🔍 Initializing advanced scanner for {}", chain_info.config.name);
    
    // Create multi-RPC provider
    let multi_rpc = Arc::new(MultiRpcProvider::new(chain_info.rpc_urls)?);
    let primary_provider = multi_rpc.get_provider();
    
    // Create advanced scanner
    let scanner = Arc::new(AdvancedScanner::new(
        primary_provider.clone(),
        chain_info.ws_url,
        chain_info.config.clone(),
    ).await?);
    
    // Create flash loan executor
    let executor = FlashLoanExecutor::new(primary_provider.clone(), chain_info.config.clone());
    
    // Phase 1: Discover all pairs (do this once at startup)
    scanner.discover_all_pairs().await?;
    
    // Phase 2: Subscribe to WebSocket updates if available
    scanner.subscribe_to_updates().await.ok();
    
    // Phase 3: Main scanning loop with intelligent updates
    let scanner_clone = scanner.clone();
    tokio::spawn(async move {
        let mut update_interval = interval(Duration::from_millis(200)); // Fast updates
        loop {
            update_interval.tick().await;
            scanner_clone.smart_update_reserves().await.ok();
        }
    });
    
    // Phase 4: Opportunity detection loop
    let mut opportunity_interval = interval(Duration::from_millis(100)); // Ultra-fast opportunity detection
    let min_profit_usd: f64 = env::var("MIN_PROFIT_USD")
        .unwrap_or_else(|_| "10".to_string()) // Lower threshold for L2s
        .parse()
        .unwrap_or(10.0);
    
    loop {
        opportunity_interval.tick().await;
        
        let opportunities = scanner.find_all_opportunities().await;
        
        for opp in opportunities.iter().take(10) { // Process top 10 opportunities
            // Calculate estimated profit
            let estimated_profit = calculate_profit(&opp, 0.001); // 0.1% slippage
            
            if estimated_profit > min_profit_usd {
                info!(
                    "💰 {} Arbitrage: {} ↔️ {} | {:.3}% spread | Est. profit: ${:.2}",
                    chain_info.config.name, opp.dex1, opp.dex2, opp.spread_pct, estimated_profit
                );
                
                // Convert to old opportunity format for executor
                let exec_opp = crate::scanner::Opportunity {
                    token0: opp.token0,
                    token1: opp.token1,
                    dex1: opp.dex1.clone(),
                    dex2: opp.dex2.clone(),
                    pair1: opp.pair1,
                    pair2: opp.pair2,
                    spread_pct: opp.spread_pct,
                    optimal_amount: calculate_optimal_trade_size(&opp),
                    profit_usd: estimated_profit,
                    gas_cost_usd: 0.5, // Approximate for L2
                    flash_loan_provider: chain_info.config.flash_loan_providers[0],
                };
                
                // Execute if profitable enough
                if estimated_profit > min_profit_usd * 2.0 { // Only execute if profit is 2x threshold
                    match executor.execute_opportunity(&exec_opp).await {
                        Ok(tx_hash) => {
                            info!("✅ Executed: {} | Profit: ${:.2}", tx_hash, estimated_profit);
                        },
                        Err(e) => {
                            warn!("❌ Execution failed: {}", e);
                        }
                    }
                }
            }
        }
    }
}

fn get_rpc_list(chain_prefix: &str) -> Vec<String> {
    let mut rpcs = Vec::new();
    
    // Get primary RPC
    if let Ok(primary) = env::var(format!("{}_RPC", chain_prefix)) {
        rpcs.push(primary);
    }
    
    // Get backup RPCs (up to 5)
    for i in 1..=5 {
        if let Ok(backup) = env::var(format!("{}_RPC_{}", chain_prefix, i)) {
            rpcs.push(backup);
        }
    }
    
    // Add public RPC as last resort
    match chain_prefix {
        "ARBITRUM" => rpcs.push("https://arb1.arbitrum.io/rpc".to_string()),
        "OPTIMISM" => rpcs.push("https://mainnet.optimism.io".to_string()),
        "BASE" => rpcs.push("https://mainnet.base.org".to_string()),
        _ => {}
    }
    
    rpcs
}

fn calculate_profit(opp: &advanced_scanner::ArbitrageOpportunity, slippage: f64) -> f64 {
    // Simplified profit calculation
    let trade_size = 1000.0; // $1000 trade
    let profit_pct = opp.spread_pct * (1.0 - slippage);
    let gross_profit = trade_size * (profit_pct / 100.0);
    let flash_fee = trade_size * 0.0009; // 0.09% flash loan fee
    let gas_cost = 0.5; // L2 gas cost estimate
    
    gross_profit - flash_fee - gas_cost
}

fn calculate_optimal_trade_size(opp: &advanced_scanner::ArbitrageOpportunity) -> U256 {
    // Calculate optimal trade size based on liquidity
    let min_reserve = opp.reserves1.0.min(opp.reserves1.1)
        .min(opp.reserves2.0).min(opp.reserves2.1);
    
    // Trade 1% of smallest reserve to minimize price impact
    min_reserve / 100
}