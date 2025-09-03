// rust-engine/src/scanner.rs
use ethers::prelude::*;
use std::sync::Arc;
use anyhow::Result;

// Import from config module
use crate::config::{ChainConfig, DexConfig};

pub struct OpportunityScanner {
    provider: Arc<Provider<Http>>,
    config: ChainConfig,
    pair_cache: Arc<dashmap::DashMap<String, PairInfo>>,
}

#[derive(Clone, Debug)]
pub struct PairInfo {
    pub token0: Address,
    pub token1: Address,
    pub pair_address: Address,
    pub dex: String,
}

#[derive(Clone, Debug)]
pub struct Opportunity {
    pub token0: Address,
    pub token1: Address,
    pub dex1: String,
    pub dex2: String,
    pub pair1: Address,
    pub pair2: Address,
    pub spread_pct: f64,
    pub optimal_amount: U256,
    pub profit_usd: f64,
    pub gas_cost_usd: f64,
    pub flash_loan_provider: Address,
}

impl OpportunityScanner {
    pub fn new(provider: Arc<Provider<Http>>, config: ChainConfig) -> Self {
        Self {
            provider,
            config,
            pair_cache: Arc::new(dashmap::DashMap::new()),
        }
    }
    
    pub async fn scan_all_pairs(&self) -> Result<Vec<Opportunity>> {
        let mut opportunities = Vec::new();
        let tokens = self.get_top_tokens();
        let gas_price = self.provider.get_gas_price().await?;
        let gas_cost_usd = self.calculate_gas_cost(gas_price).await?;
        
        for i in 0..tokens.len() {
            for j in i+1..tokens.len() {
                let token0 = tokens[i];
                let token1 = tokens[j];
                let mut prices = Vec::new();
                
                for dex in &self.config.dexes {
                    if let Ok(Some(price_data)) = self.get_pair_price(dex, token0, token1).await {
                        prices.push((dex.name.clone(), price_data));
                    }
                }
                
                for i in 0..prices.len() {
                    for j in i+1..prices.len() {
                        let (dex1, price1) = &prices[i];
                        let (dex2, price2) = &prices[j];
                        let spread = self.calculate_spread(price1.price, price2.price);
                        
                        if spread > 0.5 {
                            let optimal_amount = self.calculate_optimal_amount(
                                price1.reserves.0,
                                price1.reserves.1,
                                price2.reserves.0,
                                price2.reserves.1,
                            );
                            
                            let profit = self.calculate_profit(
                                optimal_amount,
                                spread,
                                gas_cost_usd,
                            ).await?;
                            
                            if profit > 0.0 {
                                opportunities.push(Opportunity {
                                    token0,
                                    token1,
                                    dex1: dex1.clone(),
                                    dex2: dex2.clone(),
                                    pair1: price1.pair_address,
                                    pair2: price2.pair_address,
                                    spread_pct: spread,
                                    optimal_amount,
                                    profit_usd: profit,
                                    gas_cost_usd,
                                    flash_loan_provider: self.config.flash_loan_providers[0],
                                });
                            }
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn get_pair_price(&self, dex: &DexConfig, token0: Address, token1: Address) -> Result<Option<PriceData>> {
        let cache_key = format!("{:?}-{:?}-{:?}", dex.factory, token0, token1);
        
        if let Some(pair_info) = self.pair_cache.get(&cache_key) {
            return self.fetch_reserves(pair_info.pair_address).await;
        }
        
        let factory_abi = ethers::abi::parse_abi(&[
            "function getPair(address,address) view returns (address)"
        ])?;
        
        let factory = Contract::new(dex.factory, factory_abi, self.provider.clone());
        let pair_address: Address = factory
            .method("getPair", (token0, token1))?
            .call()
            .await?;
        
        if pair_address == Address::zero() {
            return Ok(None);
        }
        
        self.pair_cache.insert(cache_key, PairInfo {
            token0,
            token1,
            pair_address,
            dex: dex.name.clone(),
        });
        
        self.fetch_reserves(pair_address).await
    }
    
    async fn fetch_reserves(&self, pair_address: Address) -> Result<Option<PriceData>> {
        let pair_abi = ethers::abi::parse_abi(&[
            "function getReserves() view returns (uint112,uint112,uint32)"
        ])?;
        
        let pair = Contract::new(pair_address, pair_abi, self.provider.clone());
        let reserves: (U256, U256, U256) = pair
            .method("getReserves", ())?
            .call()
            .await?;
        
        let price = reserves.0.as_u128() as f64 / reserves.1.as_u128().max(1) as f64;
        
        Ok(Some(PriceData {
            pair_address,
            reserves: (reserves.0, reserves.1),
            price,
        }))
    }
    
    fn calculate_spread(&self, price1: f64, price2: f64) -> f64 {
        ((price1 - price2).abs() / price1.min(price2)) * 100.0
    }
    
    fn calculate_optimal_amount(&self, r1_0: U256, _r1_1: U256, r2_0: U256, _r2_1: U256) -> U256 {
        let avg_reserve = (r1_0 + r2_0) / 2;
        avg_reserve / 100
    }
    
    async fn calculate_profit(&self, amount: U256, spread_pct: f64, gas_cost: f64) -> Result<f64> {
        let token_price_usd = 1.0;
        let trade_value = (amount.as_u128() as f64 / 1e18) * token_price_usd;
        let gross_profit = trade_value * (spread_pct / 100.0) * 0.9;
        let flash_loan_fee = trade_value * 0.0009;
        Ok(gross_profit - gas_cost - flash_loan_fee)
    }
    
    async fn calculate_gas_cost(&self, gas_price: U256) -> Result<f64> {
        let gas_units = 500_000u64;
        let eth_price = 2000.0;
        Ok((gas_price.as_u64() as f64 * gas_units as f64 * eth_price) / 1e18)
    }
    
    fn get_top_tokens(&self) -> Vec<Address> {
        match self.config.chain_id {
            42161 => vec![
                "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1".parse().unwrap(), // WETH
                "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".parse().unwrap(), // USDC
                "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9".parse().unwrap(), // USDT
            ],
            10 => vec![
                "0x4200000000000000000000000000000000000006".parse().unwrap(), // WETH
                "0x7F5c764cBc14f9669B88837ca1490cCa17c31607".parse().unwrap(), // USDC
            ],
            8453 => vec![
                "0x4200000000000000000000000000000000000006".parse().unwrap(), // WETH
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".parse().unwrap(), // USDC
            ],
            _ => vec![],
        }
    }
}

#[derive(Clone, Debug)]
struct PriceData {
    pair_address: Address,
    reserves: (U256, U256),
    price: f64,
}