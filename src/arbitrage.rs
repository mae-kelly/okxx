use crate::chains::ChainManager;
use crate::config::Config;
use crate::dexs::DexManager;
use crate::flashloan::FlashLoanManager;
use crate::types::*;
use anyhow::Result;
use rust_decimal::Decimal;
use std::sync::Arc;
use chrono::Utc;

pub struct ArbitrageEngine {
    state: Arc<SharedState>,
    chain_manager: Arc<ChainManager>,
    dex_manager: Arc<DexManager>,
    flash_loan_manager: Arc<FlashLoanManager>,
    config: Config,
}

impl ArbitrageEngine {
    pub fn new(
        state: Arc<SharedState>,
        chain_manager: Arc<ChainManager>,
        dex_manager: Arc<DexManager>,
        flash_loan_manager: Arc<FlashLoanManager>,
        config: Config,
    ) -> Self {
        Self {
            state,
            chain_manager,
            dex_manager,
            flash_loan_manager,
            config,
        }
    }
    
    pub async fn scan_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Scan each chain
        for chain in Chain::all() {
            if let Some(_provider) = self.chain_manager.get_provider(&chain) {
                // Get gas price for the chain
                let gas_price = self.state.gas_prices.get(&chain)
                    .map(|g| g.standard)
                    .unwrap_or(Decimal::from(50));
                
                // Skip if gas price is too high
                if gas_price > self.config.max_gas_price_gwei {
                    continue;
                }
                
                // Find triangular arbitrage opportunities
                if let Ok(tri_arbs) = self.find_triangular_arbitrage(&chain, gas_price).await {
                    opportunities.extend(tri_arbs);
                }
                
                // Find cross-DEX arbitrage opportunities
                if let Ok(cross_arbs) = self.find_cross_dex_arbitrage(&chain, gas_price).await {
                    opportunities.extend(cross_arbs);
                }
            }
        }
        
        // Sort by profit
        opportunities.sort_by(|a, b| b.net_profit_usd.partial_cmp(&a.net_profit_usd).unwrap());
        
        Ok(opportunities)
    }
    
    async fn find_triangular_arbitrage(
        &self,
        chain: &Chain,
        gas_price: Decimal,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get all pools for this chain
        let pools: Vec<LiquidityPool> = self.state.pools
            .iter()
            .filter(|p| p.chain == *chain)
            .map(|p| p.clone())
            .collect();
        
        // Common triangular paths: USDC -> ETH -> TOKEN -> USDC
        let base_amount = Decimal::from(10000); // Start with $10k USDC
        
        for pool1 in &pools {
            for pool2 in &pools {
                for pool3 in &pools {
                    if let Some(opp) = self.check_triangular_path(
                        chain,
                        pool1,
                        pool2,
                        pool3,
                        base_amount,
                        gas_price,
                    ).await {
                        if opp.net_profit_usd > self.config.min_profit_usd {
                            opportunities.push(opp);
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn find_cross_dex_arbitrage(
        &self,
        chain: &Chain,
        gas_price: Decimal,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        
        // Get prices from different sources
        let prices: Vec<PriceData> = self.state.prices
            .iter()
            .filter(|p| p.chain == *chain)
            .map(|p| p.clone())
            .collect();
        
        // Look for price discrepancies
        for price1 in &prices {
            for price2 in &prices {
                if price1.token_pair == price2.token_pair && price1.source != price2.source {
                    let price_diff = (price1.price - price2.price).abs();
                    let avg_price = (price1.price + price2.price) / Decimal::from(2);
                    let spread_pct = (price_diff / avg_price) * Decimal::from(100);
                    
                    if spread_pct > Decimal::from_str_exact("0.5").unwrap() {
                        if let Some(opp) = self.create_cross_dex_opportunity(
                            chain,
                            price1,
                            price2,
                            gas_price,
                        ).await {
                            if opp.net_profit_usd > self.config.min_profit_usd {
                                opportunities.push(opp);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(opportunities)
    }
    
    async fn check_triangular_path(
        &self,
        chain: &Chain,
        pool1: &LiquidityPool,
        pool2: &LiquidityPool,
        pool3: &LiquidityPool,
        initial_amount: Decimal,
        gas_price: Decimal,
    ) -> Option<ArbitrageOpportunity> {
        // Simulate the trades
        let amount1 = self.dex_manager.calculate_output_amount(
            initial_amount,
            pool1.reserve0,
            pool1.reserve1,
            pool1.fee,
        );
        
        let amount2 = self.dex_manager.calculate_output_amount(
            amount1,
            pool2.reserve0,
            pool2.reserve1,
            pool2.fee,
        );
        
        let final_amount = self.dex_manager.calculate_output_amount(
            amount2,
            pool3.reserve0,
            pool3.reserve1,
            pool3.fee,
        );
        
        // Calculate profit
        let gross_profit = final_amount - initial_amount;
        
        if gross_profit <= Decimal::ZERO {
            return None;
        }
        
        // Get best flash loan provider
        let flash_provider = self.flash_loan_manager.get_best_provider(chain, initial_amount)?;
        let flash_fee = self.flash_loan_manager.calculate_flash_loan_fee(flash_provider, initial_amount);
        
        // Calculate gas cost
        let gas_units = Decimal::from(500000); // Estimated gas for 3 swaps + flash loan
        let gas_cost_eth = gas_units * gas_price / Decimal::from(1_000_000_000);
        let eth_price = Decimal::from(2500);
        let gas_cost_usd = gas_cost_eth * eth_price;
        
        let net_profit = gross_profit - flash_fee - gas_cost_usd;
        let roi = (net_profit / initial_amount) * Decimal::from(100);
        
        if net_profit > Decimal::ZERO {
            Some(ArbitrageOpportunity {
                id: format!("{}", blake3::hash(format!("{:?}{}", chain, Utc::now()).as_bytes())),
                chain: *chain,
                opportunity_type: "Triangular".to_string(),
                path: vec![
                    TradePath {
                        dex: pool1.dex.clone(),
                        pool_address: pool1.address.clone(),
                        token_in: pool1.token0.symbol.clone(),
                        token_out: pool1.token1.symbol.clone(),
                        amount_in: initial_amount,
                        amount_out: amount1,
                    },
                    TradePath {
                        dex: pool2.dex.clone(),
                        pool_address: pool2.address.clone(),
                        token_in: pool2.token0.symbol.clone(),
                        token_out: pool2.token1.symbol.clone(),
                        amount_in: amount1,
                        amount_out: amount2,
                    },
                    TradePath {
                        dex: pool3.dex.clone(),
                        pool_address: pool3.address.clone(),
                        token_in: pool3.token0.symbol.clone(),
                        token_out: pool3.token1.symbol.clone(),
                        amount_in: amount2,
                        amount_out: final_amount,
                    },
                ],
                initial_amount,
                final_amount,
                gross_profit,
                flash_loan_provider: flash_provider.name.clone(),
                flash_loan_fee: flash_fee,
                flash_loan_fee_percentage: flash_provider.fee_percentage,
                gas_cost_usd,
                net_profit_usd: net_profit,
                roi_percentage: roi,
                confidence_score: 0.85,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }
    
    async fn create_cross_dex_opportunity(
        &self,
        chain: &Chain,
        price1: &PriceData,
        price2: &PriceData,
        gas_price: Decimal,
    ) -> Option<ArbitrageOpportunity> {
        let initial_amount = Decimal::from(10000);
        
        // Determine buy and sell prices
        let (buy_price, buy_source, sell_price, sell_source) = if price1.price < price2.price {
            (price1.price, &price1.source, price2.price, &price2.source)
        } else {
            (price2.price, &price2.source, price1.price, &price1.source)
        };
        
        // Calculate profit
        let tokens_bought = initial_amount / buy_price;
        let final_amount = tokens_bought * sell_price;
        let gross_profit = final_amount - initial_amount;
        
        // Get flash loan details
        let flash_provider = self.flash_loan_manager.get_best_provider(chain, initial_amount)?;
        let flash_fee = self.flash_loan_manager.calculate_flash_loan_fee(flash_provider, initial_amount);
        
        // Calculate gas cost
        let gas_units = Decimal::from(300000);
        let gas_cost_eth = gas_units * gas_price / Decimal::from(1_000_000_000);
        let eth_price = Decimal::from(2500);
        let gas_cost_usd = gas_cost_eth * eth_price;
        
        let net_profit = gross_profit - flash_fee - gas_cost_usd;
        let roi = (net_profit / initial_amount) * Decimal::from(100);
        
        if net_profit > Decimal::ZERO {
            Some(ArbitrageOpportunity {
                id: format!("{}", blake3::hash(format!("{:?}{}", chain, Utc::now()).as_bytes())),
                chain: *chain,
                opportunity_type: "Cross-DEX".to_string(),
                path: vec![
                    TradePath {
                        dex: buy_source.clone(),
                        pool_address: String::new(),
                        token_in: "USDC".to_string(),
                        token_out: price1.token_pair.clone(),
                        amount_in: initial_amount,
                        amount_out: tokens_bought,
                    },
                    TradePath {
                        dex: sell_source.clone(),
                        pool_address: String::new(),
                        token_in: price1.token_pair.clone(),
                        token_out: "USDC".to_string(),
                        amount_in: tokens_bought,
                        amount_out: final_amount,
                    },
                ],
                initial_amount,
                final_amount,
                gross_profit,
                flash_loan_provider: flash_provider.name.clone(),
                flash_loan_fee: flash_fee,
                flash_loan_fee_percentage: flash_provider.fee_percentage,
                gas_cost_usd,
                net_profit_usd: net_profit,
                roi_percentage: roi,
                confidence_score: 0.75,
                timestamp: Utc::now(),
            })
        } else {
            None
        }
    }
}

use rust_decimal::prelude::FromStr;