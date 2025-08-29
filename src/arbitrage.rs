use std::sync::Arc;
use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::str::FromStr;
use chrono::Utc;
use std::collections::{HashMap, HashSet, VecDeque};
use crate::types::{
    SharedState, ArbitrageOpportunity, TradeLeg, Chain, 
    FlashLoanProvider, LiquidityPool, PriceData, GasPrice
};
use crate::chains::ChainManager;
use crate::dexs::DexManager;

pub struct ArbitrageEngine {
    state: Arc<SharedState>,
    chain_manager: Arc<ChainManager>,
    dex_manager: Arc<DexManager>,
    flash_loan_providers: Vec<FlashLoanProvider>,
}

impl ArbitrageEngine {
    pub fn new(
        state: Arc<SharedState>,
        chain_manager: Arc<ChainManager>,
        dex_manager: Arc<DexManager>,
    ) -> Self {
        let flash_loan_providers = vec![
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str("0.0009").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
            },
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                chain: Chain::Arbitrum,
                fee_percentage: Decimal::from_str("0.0009").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
            },
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                chain: Chain::Optimism,
                fee_percentage: Decimal::from_str("0.0009").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
            },
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                chain: Chain::Polygon,
                fee_percentage: Decimal::from_str("0.0009").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".to_string(),
            },
            FlashLoanProvider {
                name: "dYdX".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str("0.0").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x1E0447b19BB6EcFdAe1e4AE1694b0C3659614e4e".to_string(),
            },
            FlashLoanProvider {
                name: "Balancer".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str("0.0").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
            },
            FlashLoanProvider {
                name: "Uniswap V3".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str("0.0001").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45".to_string(),
            },
            FlashLoanProvider {
                name: "PancakeSwap V3".to_string(),
                chain: Chain::BinanceSmartChain,
                fee_percentage: Decimal::from_str("0.0001").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x13f4EA83D0bd40E75C8222255bc855a974568Dd4".to_string(),
            },
        ];
        
        Self {
            state,
            chain_manager,
            dex_manager,
            flash_loan_providers,
        }
    }
    
    pub async fn scan_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        for chain in &[
            Chain::Ethereum,
            Chain::BinanceSmartChain,
            Chain::Polygon,
            Chain::Arbitrum,
            Chain::Optimism,
            Chain::Avalanche,
            Chain::Fantom,
            Chain::Base,
        ] {
            let chain_opportunities = self.scan_chain(chain).await?;
            opportunities.extend(chain_opportunities);
        }
        
        opportunities.sort_by(|a, b| b.profit_usd.partial_cmp(&a.profit_usd).unwrap());
        opportunities.truncate(100);
        
        Ok(opportunities)
    }
    
    async fn scan_chain(&self, chain: &Chain) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        let triangular_opps = self.find_triangular_arbitrage(chain).await?;
        opportunities.extend(triangular_opps);
        
        let cross_dex_opps = self.find_cross_dex_arbitrage(chain).await?;
        opportunities.extend(cross_dex_opps);
        
        let sandwich_opps = self.find_sandwich_opportunities(chain).await?;
        opportunities.extend(sandwich_opps);
        
        Ok(opportunities)
    }
    
    async fn find_triangular_arbitrage(&self, chain: &Chain) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        let pools = self.get_pools_for_chain(chain);
        
        let token_graph = self.build_token_graph(&pools);
        
        for (start_token, _) in token_graph.iter() {
            let paths = self.find_arbitrage_paths(start_token, &token_graph, 3);
            
            for path in paths {
                if let Some(opportunity) = self.calculate_path_profit(path, chain).await {
                    if opportunity.profit_usd > 50.0 {
                        opportunities.push(opportunity);
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn find_cross_dex_arbitrage(&self, chain: &Chain) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        let prices = self.get_prices_for_chain(chain);
        
        let mut token_prices: HashMap<String, Vec<(String, Decimal)>> = HashMap::new();
        
        for (_, price_data) in prices {
            let tokens: Vec<&str> = price_data.token_pair.split('/').collect();
            if tokens.len() == 2 {
                token_prices.entry(tokens[0].to_string())
                    .or_insert_with(Vec::new)
                    .push((price_data.exchange.clone(), price_data.price));
            }
        }
        
        for (token, exchanges) in token_prices {
            if exchanges.len() >= 2 {
                for i in 0..exchanges.len() {
                    for j in i+1..exchanges.len() {
                        let price_diff = (exchanges[i].1 - exchanges[j].1).abs();
                        let avg_price = (exchanges[i].1 + exchanges[j].1) / Decimal::from(2);
                        let spread_percentage = (price_diff / avg_price) * Decimal::from(100);
                        
                        if spread_percentage > Decimal::from_str("0.5").unwrap() {
                            let (buy_exchange, buy_price, sell_exchange, sell_price) = 
                                if exchanges[i].1 < exchanges[j].1 {
                                    (&exchanges[i].0, exchanges[i].1, &exchanges[j].0, exchanges[j].1)
                                } else {
                                    (&exchanges[j].0, exchanges[j].1, &exchanges[i].0, exchanges[i].1)
                                };
                            
                            let opportunity = self.create_cross_dex_opportunity(
                                &token,
                                buy_exchange,
                                sell_exchange,
                                buy_price,
                                sell_price,
                                chain,
                            ).await?;
                            
                            if opportunity.profit_usd > 50.0 {
                                opportunities.push(opportunity);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn find_sandwich_opportunities(&self, chain: &Chain) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        let pools = self.get_pools_for_chain(chain);
        let gas_price = self.state.gas_prices.get(chain).map(|g| g.clone());
        
        if let Some(gas) = gas_price {
            for pool in pools.iter() {
                let slippage_opportunity = self.calculate_sandwich_profit(pool, &gas).await;
                if let Some(opp) = slippage_opportunity {
                    if opp.profit_usd > 100.0 {
                        opportunities.push(opp);
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    fn get_pools_for_chain(&self, chain: &Chain) -> Vec<LiquidityPool> {
        self.state.liquidity_pools.iter()
            .filter(|entry| entry.chain == *chain)
            .map(|entry| entry.value().clone())
            .collect()
    }
    
    fn get_prices_for_chain(&self, chain: &Chain) -> Vec<(String, PriceData)> {
        self.state.prices.iter()
            .filter(|entry| entry.chain == *chain)
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect()
    }
    
    fn build_token_graph(&self, pools: &[LiquidityPool]) -> HashMap<String, Vec<(String, LiquidityPool)>> {
        let mut graph: HashMap<String, Vec<(String, LiquidityPool)>> = HashMap::new();
        
        for pool in pools {
            graph.entry(pool.token0.address.clone())
                .or_insert_with(Vec::new)
                .push((pool.token1.address.clone(), pool.clone()));
            
            graph.entry(pool.token1.address.clone())
                .or_insert_with(Vec::new)
                .push((pool.token0.address.clone(), pool.clone()));
        }
        
        graph
    }
    
    fn find_arbitrage_paths(
        &self,
        start_token: &str,
        graph: &HashMap<String, Vec<(String, LiquidityPool)>>,
        max_depth: usize,
    ) -> Vec<Vec<(String, String, LiquidityPool)>> {
        let mut paths = Vec::new();
        let mut queue = VecDeque::new();
        
        queue.push_back((
            start_token.to_string(),
            vec![],
            HashSet::new(),
        ));
        
        while let Some((current_token, path, visited)) = queue.pop_front() {
            if path.len() >= max_depth {
                if path.len() == max_depth && current_token == start_token && !path.is_empty() {
                    paths.push(path);
                }
                continue;
            }
            
            if let Some(neighbors) = graph.get(&current_token) {
                for (next_token, pool) in neighbors {
                    if !visited.contains(next_token) || (next_token == start_token && path.len() >= 2) {
                        let mut new_visited = visited.clone();
                        new_visited.insert(current_token.clone());
                        
                        let mut new_path = path.clone();
                        new_path.push((current_token.clone(), next_token.clone(), pool.clone()));
                        
                        queue.push_back((next_token.clone(), new_path, new_visited));
                    }
                }
            }
        }
        
        paths
    }
    
    async fn calculate_path_profit(
        &self,
        path: Vec<(String, String, LiquidityPool)>,
        chain: &Chain,
    ) -> Option<ArbitrageOpportunity> {
        let initial_amount = Decimal::from(1000);
        let mut current_amount = initial_amount;
        let mut trade_legs = Vec::new();
        let mut total_gas = Decimal::ZERO;
        
        let gas_price = self.state.gas_prices.get(chain)?;
        
        for (token_in, token_out, pool) in &path {
            let token_in_is_token0 = pool.token0.address == *token_in;
            
            let amount_out = self.dex_manager.calculate_swap_amount(
                pool,
                current_amount,
                token_in_is_token0,
            ).await.ok()?;
            
            let gas_estimate = Decimal::from(150000) * gas_price.fast / Decimal::from(1_000_000_000);
            total_gas += gas_estimate;
            
            trade_legs.push(TradeLeg {
                exchange: pool.exchange.clone(),
                pool_address: pool.address.clone(),
                token_in: token_in.clone(),
                token_out: token_out.clone(),
                amount_in: current_amount,
                amount_out,
                price: amount_out / current_amount,
                fee: pool.fee * current_amount,
                gas_estimate,
            });
            
            current_amount = amount_out;
        }
        
        let flash_loan_provider = self.get_best_flash_loan_provider(chain)?;
        let flash_loan_fee = initial_amount * flash_loan_provider.fee_percentage;
        
        let profit_amount = current_amount - initial_amount - flash_loan_fee;
        let total_cost = total_gas + flash_loan_fee;
        
        if profit_amount > total_cost {
            let profit_usd = (profit_amount - total_cost).to_f64().unwrap_or(0.0);
            let roi = ((profit_amount - total_cost) / initial_amount * Decimal::from(100))
                .to_f64().unwrap_or(0.0);
            
            Some(ArbitrageOpportunity {
                id: format!("{}", blake3::hash(format!("{:?}{}", path, Utc::now()).as_bytes())),
                path: trade_legs,
                initial_amount,
                final_amount: current_amount,
                profit_amount: profit_amount - total_cost,
                profit_usd,
                roi_percentage: roi,
                total_gas_cost: total_gas,
                flash_loan_fee,
                chain: chain.clone(),
                timestamp: Utc::now(),
                execution_time_ms: 0,
            })
        } else {
            None
        }
    }
    
    async fn create_cross_dex_opportunity(
        &self,
        token: &str,
        buy_exchange: &str,
        sell_exchange: &str,
        buy_price: Decimal,
        sell_price: Decimal,
        chain: &Chain,
    ) -> Result<ArbitrageOpportunity> {
        let initial_amount = Decimal::from(10000);
        let tokens_bought = initial_amount / buy_price;
        let final_amount = tokens_bought * sell_price;
        
        let gas_price = self.state.gas_prices.get(chain)
            .ok_or_else(|| anyhow::anyhow!("Gas price not found"))?;
        
        let gas_estimate = Decimal::from(300000) * gas_price.fast / Decimal::from(1_000_000_000);
        
        let flash_loan_provider = self.get_best_flash_loan_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("No flash loan provider"))?;
        let flash_loan_fee = initial_amount * flash_loan_provider.fee_percentage;
        
        let profit = final_amount - initial_amount - gas_estimate - flash_loan_fee;
        
        let trade_legs = vec![
            TradeLeg {
                exchange: buy_exchange.to_string(),
                pool_address: String::new(),
                token_in: "USDC".to_string(),
                token_out: token.to_string(),
                amount_in: initial_amount,
                amount_out: tokens_bought,
                price: buy_price,
                fee: initial_amount * Decimal::from_str("0.003").unwrap(),
                gas_estimate: gas_estimate / Decimal::from(2),
            },
            TradeLeg {
                exchange: sell_exchange.to_string(),
                pool_address: String::new(),
                token_in: token.to_string(),
                token_out: "USDC".to_string(),
                amount_in: tokens_bought,
                amount_out: final_amount,
                price: sell_price,
                fee: tokens_bought * Decimal::from_str("0.003").unwrap(),
                gas_estimate: gas_estimate / Decimal::from(2),
            },
        ];
        
        Ok(ArbitrageOpportunity {
            id: format!("{}", blake3::hash(format!("{}{}{}{}", token, buy_exchange, sell_exchange, Utc::now()).as_bytes())),
            path: trade_legs,
            initial_amount,
            final_amount,
            profit_amount: profit,
            profit_usd: profit.to_f64().unwrap_or(0.0),
            roi_percentage: (profit / initial_amount * Decimal::from(100)).to_f64().unwrap_or(0.0),
            total_gas_cost: gas_estimate,
            flash_loan_fee,
            chain: chain.clone(),
            timestamp: Utc::now(),
            execution_time_ms: 0,
        })
    }
    
    async fn calculate_sandwich_profit(&self, pool: &LiquidityPool, gas_price: &GasPrice) -> Option<ArbitrageOpportunity> {
        let target_trade_amount = Decimal::from(50000);
        let frontrun_amount = target_trade_amount / Decimal::from(10);
        
        let price_impact = (frontrun_amount * Decimal::from(2)) / (pool.reserve0 + pool.reserve1);
        
        if price_impact < Decimal::from_str("0.01").unwrap() {
            return None;
        }
        
        let expected_profit = target_trade_amount * price_impact * Decimal::from_str("0.5").unwrap();
        
        let gas_cost = Decimal::from(500000) * gas_price.fast / Decimal::from(1_000_000_000);
        
        let net_profit = expected_profit - gas_cost;
        
        if net_profit > Decimal::from(100) {
            Some(ArbitrageOpportunity {
                id: format!("{}", blake3::hash(format!("{}{}", pool.address, Utc::now()).as_bytes())),
                path: vec![
                    TradeLeg {
                        exchange: pool.exchange.clone(),
                        pool_address: pool.address.clone(),
                        token_in: pool.token0.address.clone(),
                        token_out: pool.token1.address.clone(),
                        amount_in: frontrun_amount,
                        amount_out: frontrun_amount,
                        price: Decimal::ONE,
                        fee: frontrun_amount * pool.fee,
                        gas_estimate: gas_cost / Decimal::from(2),
                    },
                    TradeLeg {
                        exchange: pool.exchange.clone(),
                        pool_address: pool.address.clone(),
                        token_in: pool.token1.address.clone(),
                        token_out: pool.token0.address.clone(),
                        amount_in: frontrun_amount,
                        amount_out: frontrun_amount + expected_profit,
                        price: Decimal::ONE,
                        fee: frontrun_amount * pool.fee,
                        gas_estimate: gas_cost / Decimal::from(2),
                    },
                ],
                initial_amount: frontrun_amount,
                final_amount: frontrun_amount + expected_profit,
                profit_amount: net_profit,
                profit_usd: net_profit.to_f64().unwrap_or(0.0),
                roi_percentage: (net_profit / frontrun_amount * Decimal::from(100)).to_f64().unwrap_or(0.0),
                total_gas_cost: gas_cost,
                flash_loan_fee: Decimal::ZERO,
                chain: pool.chain.clone(),
                timestamp: Utc::now(),
                execution_time_ms: 0,
            })
        } else {
            None
        }
    }
    
    fn get_best_flash_loan_provider(&self, chain: &Chain) -> Option<&FlashLoanProvider> {
        self.flash_loan_providers.iter()
            .filter(|p| p.chain == *chain)
            .min_by_key(|p| {
                let fee_in_basis_points = (p.fee_percentage * Decimal::from(10000)).to_f64().unwrap_or(f64::MAX);
                (fee_in_basis_points * 1000.0) as u64
            })
    }
}