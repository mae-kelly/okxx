// File: src/l2_scanner.rs

use ethers::prelude::*;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::time::{Duration, interval};
use dashmap::DashMap;
use futures::future::join_all;

// L2 Network Configuration
#[derive(Debug, Clone)]
pub struct L2Network {
    name: String,
    chain_id: u64,
    rpc_url: String,
    weth_address: Address,
    block_time_ms: u64,
}

// DEX Configuration for each L2
#[derive(Debug, Clone)]
pub struct L2Dex {
    name: String,
    factory: Address,
    router: Address,
    fee_bps: Vec<u32>, // Multiple fee tiers for V3
    version: DexVersion,
}

#[derive(Debug, Clone)]
enum DexVersion {
    V2,
    V3,
}

pub struct L2ArbitrageScanner {
    networks: HashMap<String, L2Network>,
    providers: HashMap<String, Arc<Provider<Http>>>,
    dexes: HashMap<String, Vec<L2Dex>>,
    pair_cache: Arc<DashMap<String, PairInfo>>,
    min_profit_usd: f64,
}

#[derive(Debug, Clone)]
struct PairInfo {
    token0: Address,
    token1: Address,
    pair_address: Address,
    reserves: (U256, U256),
    block_number: u64,
}

impl L2ArbitrageScanner {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let mut networks = HashMap::new();
        let mut providers = HashMap::new();
        let mut dexes = HashMap::new();

        // Initialize Arbitrum
        let arbitrum = L2Network {
            name: "arbitrum".to_string(),
            chain_id: 42161,
            rpc_url: "https://arb1.arbitrum.io/rpc".to_string(),
            weth_address: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".parse()?,
            block_time_ms: 250,
        };
        
        let arb_provider = Arc::new(Provider::<Http>::try_from(&arbitrum.rpc_url)?);
        providers.insert("arbitrum".to_string(), arb_provider);
        
        // Arbitrum DEXes
        let mut arb_dexes = vec![
            L2Dex {
                name: "UniswapV3".to_string(),
                factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse()?,
                router: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse()?,
                fee_bps: vec![500, 3000, 10000], // 0.05%, 0.3%, 1%
                version: DexVersion::V3,
            },
            L2Dex {
                name: "SushiswapV3".to_string(),
                factory: "0x1af415a1EbA07a4986a52B6f2e7dE7003D82231e".parse()?,
                router: "0x8A21F6768C1f8075791D08546Dadf6daA0bE820c".parse()?,
                fee_bps: vec![100, 500, 2500, 10000],
                version: DexVersion::V3,
            },
            L2Dex {
                name: "Camelot".to_string(),
                factory: "0x6EcCab422D763aC031210895C81787E87B43A652".parse()?,
                router: "0xc873fEcbd354f5A56E00E710B90EF4201db2448d".parse()?,
                fee_bps: vec![300],
                version: DexVersion::V2,
            },
            L2Dex {
                name: "TraderJoe".to_string(),
                factory: "0xaE4EC9901c3076D0DdBe76A520F9E90a6227aCB7".parse()?,
                router: "0xb4315e873dBcf96Ffd0acd8EA43f689D8c20fB30".parse()?,
                fee_bps: vec![300],
                version: DexVersion::V2,
            },
            L2Dex {
                name: "Zyberswap".to_string(),
                factory: "0xaC2ee06A14c52570Ef3B9812Ed240BCe359772e7".parse()?,
                router: "0x16e71B13fE6079B4312063F7E81F76d165Ad32Ad".parse()?,
                fee_bps: vec![100, 500, 2500],
                version: DexVersion::V3,
            },
            L2Dex {
                name: "RamsesV2".to_string(),
                factory: "0xAA2cd7477c451E703f3B9Ba5663334914763edF8".parse()?,
                router: "0xAA23611badAFB62D37E7295A682D21960ac85A90".parse()?,
                fee_bps: vec![100, 500, 3000],
                version: DexVersion::V3,
            },
        ];
        
        dexes.insert("arbitrum".to_string(), arb_dexes);
        networks.insert("arbitrum".to_string(), arbitrum);

        // Initialize Optimism
        let optimism = L2Network {
            name: "optimism".to_string(),
            chain_id: 10,
            rpc_url: "https://mainnet.optimism.io".to_string(),
            weth_address: "0x4200000000000000000000000000000000000006".parse()?,
            block_time_ms: 2000,
        };
        
        let opt_provider = Arc::new(Provider::<Http>::try_from(&optimism.rpc_url)?);
        providers.insert("optimism".to_string(), opt_provider);
        
        let mut opt_dexes = vec![
            L2Dex {
                name: "UniswapV3".to_string(),
                factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse()?,
                router: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse()?,
                fee_bps: vec![500, 3000, 10000],
                version: DexVersion::V3,
            },
            L2Dex {
                name: "Velodrome".to_string(),
                factory: "0x25CbdDb98b35ab1FF77413456B31EC81A6B6B746".parse()?,
                router: "0xa732398118DF09b50c87dE4392b77bd2e80BC862".parse()?,
                fee_bps: vec![100, 300],
                version: DexVersion::V2,
            },
        ];
        
        dexes.insert("optimism".to_string(), opt_dexes);
        networks.insert("optimism".to_string(), optimism);

        // Initialize Base
        let base = L2Network {
            name: "base".to_string(),
            chain_id: 8453,
            rpc_url: "https://mainnet.base.org".to_string(),
            weth_address: "0x4200000000000000000000000000000000000006".parse()?,
            block_time_ms: 2000,
        };
        
        let base_provider = Arc::new(Provider::<Http>::try_from(&base.rpc_url)?);
        providers.insert("base".to_string(), base_provider);
        
        let mut base_dexes = vec![
            L2Dex {
                name: "UniswapV3".to_string(),
                factory: "0x33128a8fC17869897dcE68Ed026d694621f6FDfD".parse()?,
                router: "0x2626664c2603336E57B271c5C0b26F421741e481".parse()?,
                fee_bps: vec![500, 3000, 10000],
                version: DexVersion::V3,
            },
            L2Dex {
                name: "BaseSwap".to_string(),
                factory: "0xFDa619b6d20975be80A10332dD640503C9957FF8".parse()?,
                router: "0x327Df1E6de05895d2ab08513aaDD9313Fe505d86".parse()?,
                fee_bps: vec![250],
                version: DexVersion::V2,
            },
            L2Dex {
                name: "Aerodrome".to_string(),
                factory: "0x420DD381b31aEf6683db6B902084cB0FFECe40Da".parse()?,
                router: "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43".parse()?,
                fee_bps: vec![100, 300],
                version: DexVersion::V2,
            },
        ];
        
        dexes.insert("base".to_string(), base_dexes);
        networks.insert("base".to_string(), base);

        // Initialize Polygon zkEVM
        let polygon_zkevm = L2Network {
            name: "polygon_zkevm".to_string(),
            chain_id: 1101,
            rpc_url: "https://zkevm-rpc.com".to_string(),
            weth_address: "0x4F9A0e7FD2Bf6067db6994CF12E4495Df938E6e9".parse()?,
            block_time_ms: 2000,
        };
        
        let zkevm_provider = Arc::new(Provider::<Http>::try_from(&polygon_zkevm.rpc_url)?);
        providers.insert("polygon_zkevm".to_string(), zkevm_provider);
        
        let mut zkevm_dexes = vec![
            L2Dex {
                name: "QuickswapV3".to_string(),
                factory: "0x4B9f4d2435Ef65559567e5DbFC1BbB37abC43B57".parse()?,
                router: "0xf6Ad3CcF71Abb3E12beCf6b3D2a74C963859ADCd".parse()?,
                fee_bps: vec![500, 3000],
                version: DexVersion::V3,
            },
        ];
        
        dexes.insert("polygon_zkevm".to_string(), zkevm_dexes);
        networks.insert("polygon_zkevm".to_string(), polygon_zkevm);

        Ok(Self {
            networks,
            providers,
            dexes,
            pair_cache: Arc::new(DashMap::new()),
            min_profit_usd: 5.0, // Minimum $5 profit after gas
        })
    }

    pub async fn discover_all_pairs(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔍 Discovering all trading pairs across L2 networks...");
        
        for (network_name, network_dexes) in &self.dexes {
            let provider = &self.providers[network_name];
            
            for dex in network_dexes {
                println!("  Scanning {} on {}", dex.name, network_name);
                
                match dex.version {
                    DexVersion::V2 => {
                        self.discover_v2_pairs(provider.clone(), &dex).await?;
                    },
                    DexVersion::V3 => {
                        self.discover_v3_pairs(provider.clone(), &dex).await?;
                    }
                }
            }
        }
        
        println!("✅ Found {} total pairs", self.pair_cache.len());
        Ok(())
    }

    async fn discover_v2_pairs(
        &self,
        provider: Arc<Provider<Http>>,
        dex: &L2Dex
    ) -> Result<(), Box<dyn std::error::Error>> {
        let factory_abi = ethers::abi::parse_abi(&[
            "function allPairs(uint256) view returns (address)",
            "function allPairsLength() view returns (uint256)",
        ])?;
        
        let factory = Contract::new(dex.factory, factory_abi, provider);
        
        let length: U256 = factory
            .method("allPairsLength", ())?
            .call()
            .await?;
        
        let pairs_to_check = length.as_u64().min(1000); // Check first 1000 pairs
        
        for i in 0..pairs_to_check {
            let pair_address: Address = factory
                .method("allPairs", U256::from(i))?
                .call()
                .await?;
                
            // Store pair info
            let key = format!("{}_{}_pair_{}", dex.name, dex.factory, i);
            self.pair_cache.insert(key, PairInfo {
                token0: Address::zero(),
                token1: Address::zero(),
                pair_address,
                reserves: (U256::zero(), U256::zero()),
                block_number: 0,
            });
        }
        
        Ok(())
    }

    async fn discover_v3_pairs(
        &self,
        provider: Arc<Provider<Http>>,
        dex: &L2Dex
    ) -> Result<(), Box<dyn std::error::Error>> {
        // V3 pools are created on demand, so we'll check common pairs
        let common_tokens = vec![
            "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", // WETH
            "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", // USDC
            "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", // USDT
            "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f", // WBTC
            "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1", // DAI
        ];
        
        for i in 0..common_tokens.len() {
            for j in i+1..common_tokens.len() {
                let token0: Address = common_tokens[i].parse()?;
                let token1: Address = common_tokens[j].parse()?;
                
                for fee in &dex.fee_bps {
                    let pool = self.get_v3_pool_address(provider.clone(), dex.factory, token0, token1, *fee).await;
                    if let Ok(pool_addr) = pool {
                        if pool_addr != Address::zero() {
                            let key = format!("{}_{}_{}_{}_{}", dex.name, token0, token1, fee, dex.factory);
                            self.pair_cache.insert(key, PairInfo {
                                token0,
                                token1,
                                pair_address: pool_addr,
                                reserves: (U256::zero(), U256::zero()),
                                block_number: 0,
                            });
                        }
                    }
                }
            }
        }
        
        Ok(())
    }

    async fn get_v3_pool_address(
        &self,
        provider: Arc<Provider<Http>>,
        factory: Address,
        token0: Address,
        token1: Address,
        fee: u32
    ) -> Result<Address, Box<dyn std::error::Error>> {
        let factory_abi = ethers::abi::parse_abi(&[
            "function getPool(address,address,uint24) view returns (address)",
        ])?;
        
        let factory_contract = Contract::new(factory, factory_abi, provider);
        
        let pool: Address = factory_contract
            .method("getPool", (token0, token1, fee))?
            .call()
            .await?;
            
        Ok(pool)
    }

    pub async fn scan_opportunities(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting multi-chain L2 arbitrage scanner...");
        let mut interval = interval(Duration::from_millis(500));
        
        loop {
            interval.tick().await;
            
            // Scan each network in parallel
            let mut handles = vec![];
            
            for (network_name, network_config) in &self.networks {
                let network_name = network_name.clone();
                let provider = self.providers[&network_name].clone();
                let network_dexes = self.dexes[&network_name].clone();
                let cache = self.pair_cache.clone();
                let min_profit = self.min_profit_usd;
                
                let handle = tokio::spawn(async move {
                    Self::scan_network(
                        network_name,
                        provider,
                        network_dexes,
                        cache,
                        min_profit
                    ).await
                });
                
                handles.push(handle);
            }
            
            let results = join_all(handles).await;
            
            for result in results {
                if let Ok(Ok(opportunities)) = result {
                    for opp in opportunities {
                        self.display_opportunity(opp);
                    }
                }
            }
        }
    }

    async fn scan_network(
        network_name: String,
        provider: Arc<Provider<Http>>,
        dexes: Vec<L2Dex>,
        cache: Arc<DashMap<String, PairInfo>>,
        min_profit_usd: f64
    ) -> Result<Vec<ArbitrageOpportunity>, anyhow::Error> {
        let mut opportunities = Vec::new();
        let block = provider.get_block_number().await?;
        
        // Get gas price for this network
        let gas_price = provider.get_gas_price().await?;
        let gas_cost = Self::calculate_gas_cost(&network_name, gas_price);
        
        // Check pairs between different DEXes
        for i in 0..dexes.len() {
            for j in i+1..dexes.len() {
                let dex1 = &dexes[i];
                let dex2 = &dexes[j];
                
                // Find common pairs between DEXes
                for entry in cache.iter() {
                    let key = entry.key();
                    if key.contains(&dex1.name) {
                        // Check if same pair exists on dex2
                        let pair_info = entry.value();
                        
                        // Get prices from both DEXes
                        let price1 = Self::get_pair_price(
                            provider.clone(),
                            pair_info.pair_address,
                            &dex1.version
                        ).await;
                        
                        // Find corresponding pair on dex2
                        let dex2_key = key.replace(&dex1.name, &dex2.name);
                        if let Some(dex2_pair) = cache.get(&dex2_key) {
                            let price2 = Self::get_pair_price(
                                provider.clone(),
                                dex2_pair.pair_address,
                                &dex2.version
                            ).await;
                            
                            if let (Ok(p1), Ok(p2)) = (price1, price2) {
                                // Calculate spread
                                let spread_pct = ((p2 - p1).abs() / p1.min(p2)) * 100.0;
                                
                                // Skip unrealistic spreads
                                if spread_pct > 20.0 || spread_pct < 0.01 {
                                    continue;
                                }
                                
                                // Calculate fees
                                let fee1_pct = dex1.fee_bps[0] as f64 / 10000.0 * 100.0;
                                let fee2_pct = dex2.fee_bps[0] as f64 / 10000.0 * 100.0;
                                let total_fees = fee1_pct + fee2_pct;
                                
                                // Net profit after fees
                                let net_spread = spread_pct - total_fees;
                                
                                if net_spread > 0.01 {
                                    // Calculate profit on $10k trade
                                    let trade_amount_usd = 10000.0;
                                    let gross_profit = trade_amount_usd * (net_spread / 100.0);
                                    let net_profit = gross_profit - gas_cost;
                                    
                                    if net_profit > min_profit_usd {
                                        opportunities.push(ArbitrageOpportunity {
                                            network: network_name.clone(),
                                            dex_buy: if p1 < p2 { dex1.name.clone() } else { dex2.name.clone() },
                                            dex_sell: if p1 < p2 { dex2.name.clone() } else { dex1.name.clone() },
                                            token0: pair_info.token0,
                                            token1: pair_info.token1,
                                            spread_pct,
                                            net_spread,
                                            gas_cost,
                                            net_profit,
                                            block_number: block.as_u64(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }

    async fn get_pair_price(
        provider: Arc<Provider<Http>>,
        pair_address: Address,
        version: &DexVersion
    ) -> Result<f64, Box<dyn std::error::Error>> {
        match version {
            DexVersion::V2 => {
                let pair_abi = ethers::abi::parse_abi(&[
                    "function getReserves() view returns (uint112,uint112,uint32)",
                ])?;
                
                let pair = Contract::new(pair_address, pair_abi, provider);
                let reserves: (U256, U256, U256) = pair.method("getReserves", ())?.call().await?;
                
                if reserves.0 > U256::zero() && reserves.1 > U256::zero() {
                    Ok(reserves.0.as_u128() as f64 / reserves.1.as_u128() as f64)
                } else {
                    Err("Zero liquidity".into())
                }
            },
            DexVersion::V3 => {
                let pool_abi = ethers::abi::parse_abi(&[
                    "function slot0() view returns (uint160,int24,uint16,uint16,uint16,uint8,bool)",
                ])?;
                
                let pool = Contract::new(pair_address, pool_abi, provider);
                let slot0: (U256, i32, u16, u16, u16, u8, bool) = pool.method("slot0", ())?.call().await?;
                
                let sqrt_price = slot0.0;
                let price = (sqrt_price.as_u128() as f64 / (1u128 << 96) as f64).powi(2);
                Ok(price)
            }
        }
    }

    fn calculate_gas_cost(network: &str, gas_price: U256) -> f64 {
        let gas_units = match network {
            "arbitrum" => 250_000,
            "optimism" => 200_000,
            "base" => 180_000,
            "polygon_zkevm" => 300_000,
            _ => 250_000,
        };
        
        let eth_price = 2000.0; // Hardcoded ETH price, should fetch from oracle
        (gas_price.as_u128() as f64 * gas_units as f64 * eth_price) / 1e18
    }

    fn display_opportunity(&self, opp: ArbitrageOpportunity) {
        println!("\n💰 ARBITRAGE OPPORTUNITY DETECTED!");
        println!("  Network: {}", opp.network);
        println!("  Route: {} → {}", opp.dex_buy, opp.dex_sell);
        println!("  Tokens: {:?} ↔ {:?}", opp.token0, opp.token1);
        println!("  Spread: {:.4}%", opp.spread_pct);
        println!("  Net Spread (after fees): {:.4}%", opp.net_spread);
        println!("  Gas Cost: ${:.2}", opp.gas_cost);
        println!("  Net Profit (on $10k): ${:.2}", opp.net_profit);
        println!("  Block: {}", opp.block_number);
    }
}

#[derive(Debug)]
struct ArbitrageOpportunity {
    network: String,
    dex_buy: String,
    dex_sell: String,
    token0: Address,
    token1: Address,
    spread_pct: f64,
    net_spread: f64,
    gas_cost: f64,
    net_profit: f64,
    block_number: u64,
}