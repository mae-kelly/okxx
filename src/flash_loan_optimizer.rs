use std::sync::Arc;
use anyhow::Result;
use rust_decimal::Decimal;
use std::collections::HashMap;
use crate::types::{Chain, FlashLoanProvider};

pub struct FlashLoanOptimizer {
    providers: Vec<FlashLoanProvider>,
    cached_rates: HashMap<String, Decimal>,
}

impl FlashLoanOptimizer {
    pub fn new() -> Self {
        let providers = vec![
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str_exact("0.0009").unwrap(),
                max_loan_amount: HashMap::new(),
                contract_address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
            },
            FlashLoanProvider {
                name: "Balancer".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::ZERO,
                max_loan_amount: HashMap::new(),
                contract_address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
            },
            FlashLoanProvider {
                name: "dYdX".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str_exact("0.0002").unwrap(),
                max_loan_amount: HashMap::new(),
                contract_address: "0x1E0447b19BB6EcFdAe1e4AE1694b0C3659614e4e".to_string(),
            },
            FlashLoanProvider {
                name: "Uniswap V3".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str_exact("0.0001").unwrap(),
                max_loan_amount: HashMap::new(),
                contract_address: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45".to_string(),
            },
        ];
        
        Self {
            providers,
            cached_rates: HashMap::new(),
        }
    }
    
    pub fn get_best_provider(&self, chain: &Chain, amount: Decimal) -> Option<&FlashLoanProvider> {
        self.providers.iter()
            .filter(|p| p.chain == *chain)
            .filter(|p| self.can_provide_loan(p, amount))
            .min_by_key(|p| {
                let fee_in_basis_points = (p.fee_percentage * Decimal::from(10000))
                    .to_f64()
                    .unwrap_or(f64::MAX);
                (fee_in_basis_points * 1000.0) as u64
            })
    }
    
    pub fn calculate_optimal_loan_amount(
        &self,
        expected_profit: Decimal,
        provider: &FlashLoanProvider,
    ) -> Decimal {
        // Calculate optimal loan amount based on expected profit and fee
        let fee_rate = provider.fee_percentage;
        
        // Simple formula: loan_amount = expected_profit / (roi - fee_rate)
        // Assuming a target ROI of 2%
        let target_roi = Decimal::from_str_exact("0.02").unwrap();
        
        if target_roi > fee_rate {
            expected_profit / (target_roi - fee_rate)
        } else {
            Decimal::ZERO
        }
    }
    
    pub fn estimate_total_cost(
        &self,
        provider: &FlashLoanProvider,
        amount: Decimal,
        gas_cost: Decimal,
    ) -> Decimal {
        let loan_fee = amount * provider.fee_percentage;
        loan_fee + gas_cost
    }
    
    pub fn find_multi_provider_strategy(
        &self,
        chain: &Chain,
        total_amount: Decimal,
    ) -> Vec<(FlashLoanProvider, Decimal)> {
        let mut strategy = Vec::new();
        let mut remaining = total_amount;
        
        // Sort providers by fee
        let mut chain_providers: Vec<_> = self.providers.iter()
            .filter(|p| p.chain == *chain)
            .collect();
        
        chain_providers.sort_by_key(|p| {
            (p.fee_percentage * Decimal::from(10000))
                .to_f64()
                .unwrap_or(f64::MAX) as u64
        });
        
        // Allocate loans starting with cheapest provider
        for provider in chain_providers {
            if remaining <= Decimal::ZERO {
                break;
            }
            
            let max_from_provider = self.get_max_loan_amount(provider);
            let loan_amount = remaining.min(max_from_provider);
            
            if loan_amount > Decimal::ZERO {
                strategy.push((provider.clone(), loan_amount));
                remaining -= loan_amount;
            }
        }
        
        strategy
    }
    
    fn can_provide_loan(&self, provider: &FlashLoanProvider, amount: Decimal) -> bool {
        let max_amount = self.get_max_loan_amount(provider);
        amount <= max_amount
    }
    
    fn get_max_loan_amount(&self, provider: &FlashLoanProvider) -> Decimal {
        // In production, this would query on-chain data
        match provider.name.as_str() {
            "Aave V3" => Decimal::from(10_000_000),
            "Balancer" => Decimal::from(5_000_000),
            "dYdX" => Decimal::from(2_000_000),
            "Uniswap V3" => Decimal::from(1_000_000),
            _ => Decimal::from(100_000),
        }
    }
    
    pub fn calculate_break_even_profit(&self, provider: &FlashLoanProvider, gas_cost: Decimal) -> Decimal {
        // Minimum profit needed to break even
        gas_cost / (Decimal::ONE - provider.fee_percentage)
    }
    
    pub fn rank_providers_by_profitability(
        &self,
        chain: &Chain,
        loan_amount: Decimal,
        expected_profit: Decimal,
    ) -> Vec<(String, Decimal)> {
        let mut rankings = Vec::new();
        
        for provider in &self.providers {
            if provider.chain != *chain {
                continue;
            }
            
            let fee = loan_amount * provider.fee_percentage;
            let net_profit = expected_profit - fee;
            
            if net_profit > Decimal::ZERO {
                rankings.push((provider.name.clone(), net_profit));
            }
        }
        
        rankings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        rankings
    }
}

use rust_decimal::prelude::{FromStr, ToPrimitive};