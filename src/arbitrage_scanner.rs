use std::sync::Arc;
use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal::prelude::{ToPrimitive, FromStr};
use dashmap::DashMap;
use chrono::Utc;
use crate::types::{
    SharedState, ArbitrageOpportunity, TradeLeg, Chain, 
    MarketSignal, TokenPair, Token, SignalType
};

pub struct ArbitrageScanner {
    state: Arc<SharedState>,
}

impl ArbitrageScanner {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }
    
    pub async fn scan_for_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Scan different chains
        for chain in &[
            Chain::Ethereum,
            Chain::BinanceSmartChain,
            Chain::Polygon,
            Chain::Arbitrum,
            Chain::Optimism,
        ] {
            let chain_opportunities = self.scan_chain(chain).await?;
            opportunities.extend(chain_opportunities);
        }
        
        // Store opportunities
        let mut opps = self.state.opportunities.write().await;
        *opps = opportunities.clone();
        
        // Generate signals
        let pair_signals: DashMap<String, Vec<MarketSignal>> = DashMap::new();
        
        for opp in &opportunities {
            if opp.profit_usd > 50.0 {
                let signal = MarketSignal {
                    pair: format!("{}-{}", 
                        opp.token_pair.base.symbol, 
                        opp.token_pair.quote.symbol
                    ),
                    signal_type: SignalType::Buy,
                    strength: opp.ml_confidence,
                    timestamp: Utc::now(),
                    profit: opp.profit_amount,
                    buy_exchange: opp.buy_exchange.clone(),
                    sell_exchange: opp.sell_exchange.clone(),
                    roi: opp.roi_percentage,
                };
                
                pair_signals.entry(signal.pair.clone())
                    .or_insert_with(Vec::new)
                    .push(signal.clone());
                
                self.state.performance_stats.insert(
                    format!("{}_{}", signal.pair, Utc::now().timestamp()),
                    signal
                );
            }
        }
        
        Ok(opportunities)
    }
    
    async fn scan_chain(&self, chain: &Chain) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get prices for this chain
        let prices: Vec<_> = self.state.prices.iter()
            .filter(|entry| entry.chain == *chain)
            .map(|entry| entry.value().clone())
            .collect();
        
        // Simple cross-exchange arbitrage detection
        for i in 0..prices.len() {
            for j in i+1..prices.len() {
                if prices[i].token_pair == prices[j].token_pair {
                    let spread = ((prices[i].price - prices[j].price).abs() / prices[i].price) * Decimal::from(100);
                    
                    if spread > Decimal::from(1) {
                        let opportunity = self.create_opportunity(
                            &prices[i],
                            &prices[j],
                            chain,
                        ).await?;
                        
                        if opportunity.profit_usd > 10.0 {
                            opportunities.push(opportunity);
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn create_opportunity(
        &self,
        price1: &crate::types::PriceData,
        price2: &crate::types::PriceData,
        chain: &Chain,
    ) -> Result<ArbitrageOpportunity> {
        let initial_amount = Decimal::from(1000);
        
        let (buy_price, buy_exchange, sell_price, sell_exchange) = 
            if price1.price < price2.price {
                (price1.price, &price1.exchange, price2.price, &price2.exchange)
            } else {
                (price2.price, &price2.exchange, price1.price, &price1.exchange)
            };
        
        let tokens_bought = initial_amount / buy_price;
        let final_amount = tokens_bought * sell_price;
        let profit = final_amount - initial_amount;
        
        // Get gas price
        let gas_price = self.state.gas_prices.get(chain)
            .map(|g| g.fast)
            .unwrap_or(Decimal::from(30));
        
        let gas_cost = Decimal::from(300000) * gas_price / Decimal::from(1_000_000_000);
        let net_profit = profit - gas_cost;
        
        let token_pair = TokenPair {
            base: Token {
                address: String::new(),
                symbol: price1.token_pair.split('/').next().unwrap_or("UNKNOWN").to_string(),
                decimals: 18,
                chain_id: 1,
            },
            quote: Token {
                address: String::new(),
                symbol: price1.token_pair.split('/').nth(1).unwrap_or("USDT").to_string(),
                decimals: 18,
                chain_id: 1,
            },
        };
        
        Ok(ArbitrageOpportunity {
            id: format!("{}", Utc::now().timestamp_nanos()),
            path: vec![
                TradeLeg {
                    exchange: buy_exchange.clone(),
                    pool_address: String::new(),
                    token_in: "USDT".to_string(),
                    token_out: token_pair.base.symbol.clone(),
                    amount_in: initial_amount,
                    amount_out: tokens_bought,
                    price: buy_price,
                    fee: initial_amount * Decimal::from_str("0.003")?,
                    gas_estimate: gas_cost / Decimal::from(2),
                },
                TradeLeg {
                    exchange: sell_exchange.clone(),
                    pool_address: String::new(),
                    token_in: token_pair.base.symbol.clone(),
                    token_out: "USDT".to_string(),
                    amount_in: tokens_bought,
                    amount_out: final_amount,
                    price: sell_price,
                    fee: tokens_bought * Decimal::from_str("0.003")?,
                    gas_estimate: gas_cost / Decimal::from(2),
                },
            ],
            initial_amount,
            final_amount,
            profit_amount: profit,
            profit_usd: profit.to_f64().unwrap_or(0.0),
            roi_percentage: (profit / initial_amount * Decimal::from(100)).to_f64().unwrap_or(0.0),
            total_gas_cost: gas_cost,
            flash_loan_fee: Decimal::ZERO,
            chain: chain.clone(),
            timestamp: Utc::now(),
            execution_time_ms: 0,
            buy_exchange: buy_exchange.clone(),
            sell_exchange: sell_exchange.clone(),
            net_profit,
            ml_confidence: 0.75,
            token_pair,
        })
    }
    
    async fn calculate_triangular_arbitrage(
        &self,
        chain: &Chain,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get liquidity pools for this chain
        let pools: Vec<_> = self.state.liquidity_pools.iter()
            .filter(|entry| entry.chain == *chain)
            .map(|entry| entry.value().clone())
            .collect();
        
        // Find triangular arbitrage paths
        for i in 0..pools.len().min(50) {
            for j in i+1..pools.len().min(50) {
                for k in j+1..pools.len().min(50) {
                    if let Some(opp) = self.check_triangular_path(
                        &pools[i],
                        &pools[j],
                        &pools[k],
                        chain,
                    ).await {
                        if opp.profit_usd > 50.0 {
                            opportunities.push(opp);
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn check_triangular_path(
        &self,
        pool1: &crate::types::LiquidityPool,
        pool2: &crate::types::LiquidityPool,
        pool3: &crate::types::LiquidityPool,
        chain: &Chain,
    ) -> Option<ArbitrageOpportunity> {
        // Check if pools form a valid triangle
        let tokens = vec![
            &pool1.token0.symbol,
            &pool1.token1.symbol,
            &pool2.token0.symbol,
            &pool2.token1.symbol,
            &pool3.token0.symbol,
            &pool3.token1.symbol,
        ];
        
        // Simple validation - ensure we have exactly 3 unique tokens
        let unique_tokens: std::collections::HashSet<_> = tokens.into_iter().collect();
        if unique_tokens.len() != 3 {
            return None;
        }
        
        let initial_amount = Decimal::from(1000);
        let mut current_amount = initial_amount;
        
        // Simulate trades through the triangle
        // This is simplified - real implementation would need proper routing
        current_amount = self.simulate_swap(current_amount, pool1);
        current_amount = self.simulate_swap(current_amount, pool2);
        current_amount = self.simulate_swap(current_amount, pool3);
        
        let gross_profit = current_amount - initial_amount;
        
        // Calculate costs
        let gas_price = self.state.gas_prices.get(chain)
            .map(|g| g.fast)
            .unwrap_or(Decimal::from(30));
        
        let gas_cost = Decimal::from(450000) * gas_price / Decimal::from(1_000_000_000);
        let net_profit = gross_profit - gas_cost;
        
        if net_profit > Decimal::from(10) {
            let token_pair = TokenPair {
                base: Token {
                    address: pool1.token0.address.clone(),
                    symbol: pool1.token0.symbol.clone(),
                    decimals: pool1.token0.decimals,
                    chain_id: 1,
                },
                quote: Token {
                    address: pool1.token1.address.clone(),
                    symbol: pool1.token1.symbol.clone(),
                    decimals: pool1.token1.decimals,
                    chain_id: 1,
                },
            };
            
            Some(ArbitrageOpportunity {
                id: format!("{}", Utc::now().timestamp_nanos()),
                path: vec![
                    TradeLeg {
                        exchange: pool1.exchange.clone(),
                        pool_address: pool1.address.clone(),
                        token_in: pool1.token0.symbol.clone(),
                        token_out: pool1.token1.symbol.clone(),
                        amount_in: initial_amount,
                        amount_out: current_amount,
                        price: Decimal::ONE,
                        fee: pool1.fee * initial_amount,
                        gas_estimate: gas_cost / Decimal::from(3),
                    },
                ],
                initial_amount,
                final_amount: current_amount,
                profit_amount: gross_profit,
                profit_usd: gross_profit.to_f64().unwrap_or(0.0),
                roi_percentage: (gross_profit / initial_amount * Decimal::from(100)).to_f64().unwrap_or(0.0),
                total_gas_cost: gas_cost,
                flash_loan_fee: Decimal::ZERO,
                chain: chain.clone(),
                timestamp: Utc::now(),
                execution_time_ms: 0,
                buy_exchange: pool1.exchange.clone(),
                sell_exchange: pool3.exchange.clone(),
                net_profit,
                ml_confidence: 0.65,
                token_pair,
            })
        } else {
            None
        }
    }
    
    fn simulate_swap(&self, amount_in: Decimal, pool: &crate::types::LiquidityPool) -> Decimal {
        // Simplified AMM formula
        let amount_in_with_fee = amount_in * (Decimal::from(1) - pool.fee);
        let numerator = amount_in_with_fee * pool.reserve1;
        let denominator = pool.reserve0 + amount_in_with_fee;
        
        if denominator > Decimal::ZERO {
            numerator / denominator
        } else {
            Decimal::ZERO
        }
    }
}