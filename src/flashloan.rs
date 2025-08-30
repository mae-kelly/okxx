use std::sync::Arc;
use anyhow::Result;
use rust_decimal::Decimal;
use crate::chains::ChainManager;
use crate::types::{Chain, FlashLoanProvider};
use std::str::FromStr;

pub struct FlashLoanManager {
    chain_manager: Arc<ChainManager>,
    providers: Vec<FlashLoanProvider>,
}

impl FlashLoanManager {
    pub fn new(chain_manager: Arc<ChainManager>) -> Self {
        let providers = vec![
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::from_str("0.0009").unwrap(),
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
            },
            FlashLoanProvider {
                name: "Balancer".to_string(),
                chain: Chain::Ethereum,
                fee_percentage: Decimal::ZERO,
                max_loan_amount: std::collections::HashMap::new(),
                contract_address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
            },
        ];
        
        Self {
            chain_manager,
            providers,
        }
    }
    
    pub fn get_providers_for_chain(&self, chain: &Chain) -> Vec<&FlashLoanProvider> {
        self.providers
            .iter()
            .filter(|p| p.chain == *chain)
            .collect()
    }
    
    pub fn calculate_fee(&self, provider: &FlashLoanProvider, amount: Decimal) -> Decimal {
        amount * provider.fee_percentage
    }
}