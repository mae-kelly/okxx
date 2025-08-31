use crate::chains::ChainManager;
use crate::config::Config;
use crate::types::{Chain, FlashLoanProvider};
use anyhow::Result;
use ethers::prelude::*;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;

// Aave V3 Pool ABI
abigen!(
    IAaveV3Pool,
    r#"[
        function flashLoan(
            address receiverAddress,
            address[] calldata assets,
            uint256[] calldata amounts,
            uint256[] calldata modes,
            address onBehalfOf,
            bytes calldata params,
            uint16 referralCode
        ) external
        function FLASHLOAN_PREMIUM_TOTAL() external view returns (uint128)
    ]"#
);

// Balancer Vault ABI
abigen!(
    IBalancerVault,
    r#"[
        function flashLoan(
            address recipient,
            address[] memory tokens,
            uint256[] memory amounts,
            bytes memory userData
        ) external
    ]"#
);

pub struct FlashLoanManager {
    chain_manager: Arc<ChainManager>,
    providers: HashMap<String, FlashLoanProvider>,
}

impl FlashLoanManager {
    pub async fn new(config: &Config, chain_manager: Arc<ChainManager>) -> Result<Self> {
        let mut providers = HashMap::new();
        
        for fl_config in &config.flash_loan_providers {
            if fl_config.enabled {
                let chain = match fl_config.chain.as_str() {
                    "ethereum" => Chain::Ethereum,
                    "bsc" => Chain::BinanceSmartChain,
                    "polygon" => Chain::Polygon,
                    "arbitrum" => Chain::Arbitrum,
                    _ => continue,
                };
                
                let provider = FlashLoanProvider {
                    name: fl_config.name.clone(),
                    chain,
                    contract_address: fl_config.contract_address.clone(),
                    fee_percentage: fl_config.fee_percentage,
                    available_tokens: vec![
                        "USDC".to_string(),
                        "USDT".to_string(),
                        "DAI".to_string(),
                        "WETH".to_string(),
                        "WBTC".to_string(),
                    ],
                };
                
                providers.insert(
                    format!("{}_{}", fl_config.name, fl_config.chain),
                    provider,
                );
            }
        }
        
        Ok(Self {
            chain_manager,
            providers,
        })
    }
    
    pub fn get_best_provider(&self, chain: &Chain, amount: Decimal) -> Option<&FlashLoanProvider> {
        self.providers
            .values()
            .filter(|p| p.chain == *chain)
            .min_by_key(|p| (p.fee_percentage * Decimal::from(10000)).to_u64().unwrap_or(u64::MAX))
    }
    
    pub fn calculate_flash_loan_fee(&self, provider: &FlashLoanProvider, amount: Decimal) -> Decimal {
        amount * provider.fee_percentage
    }
    
    pub fn get_providers_for_chain(&self, chain: &Chain) -> Vec<&FlashLoanProvider> {
        self.providers
            .values()
            .filter(|p| p.chain == *chain)
            .collect()
    }
    
    pub async fn check_liquidity(
        &self,
        provider: &FlashLoanProvider,
        token_address: &str,
        amount: Decimal,
    ) -> Result<bool> {
        // In production, check actual liquidity on-chain
        // For now, assume sufficient liquidity
        Ok(true)
    }
    
    pub fn estimate_total_cost(
        &self,
        provider: &FlashLoanProvider,
        amount: Decimal,
        gas_price_gwei: Decimal,
    ) -> Decimal {
        let flash_loan_fee = self.calculate_flash_loan_fee(provider, amount);
        
        // Estimate gas cost (assuming 300k gas units for flash loan)
        let gas_units = Decimal::from(300000);
        let gas_cost_eth = gas_units * gas_price_gwei / Decimal::from(1_000_000_000);
        let eth_price = Decimal::from(2500); // In production, fetch actual price
        let gas_cost_usd = gas_cost_eth * eth_price;
        
        flash_loan_fee + gas_cost_usd
    }
}

use rust_decimal::prelude::FromStr;