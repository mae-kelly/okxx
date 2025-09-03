use anyhow::Result;
use ethers::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use std::sync::Arc;
use std::str::FromStr;

// Fixed ABI format - proper JSON structure
abigen!(
    IAaveV3Pool,
    r#"[
        {
            "name": "flashLoan",
            "type": "function",
            "inputs": [
                {"name": "receiverAddress", "type": "address"},
                {"name": "assets", "type": "address[]"},
                {"name": "amounts", "type": "uint256[]"},
                {"name": "interestRateModes", "type": "uint256[]"},
                {"name": "onBehalfOf", "type": "address"},
                {"name": "params", "type": "bytes"},
                {"name": "referralCode", "type": "uint16"}
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        },
        {
            "name": "FLASHLOAN_PREMIUM_TOTAL",
            "type": "function",
            "inputs": [],
            "outputs": [{"name": "", "type": "uint128"}],
            "stateMutability": "view"
        }
    ]"#
);

abigen!(
    IBalancerVault,
    r#"[
        {
            "name": "flashLoan",
            "type": "function",
            "inputs": [
                {"name": "recipient", "type": "address"},
                {"name": "tokens", "type": "address[]"},
                {"name": "amounts", "type": "uint256[]"},
                {"name": "userData", "type": "bytes"}
            ],
            "outputs": [],
            "stateMutability": "nonpayable"
        }
    ]"#
);

#[derive(Debug, Clone)]
pub struct FlashLoanProvider {
    pub name: String,
    pub address: String,
    pub fee_percentage: Decimal,
    pub max_loan_amount: Decimal,
    pub supported_tokens: Vec<String>,
}

pub struct FlashLoanManager {
    providers: Vec<FlashLoanProvider>,
    provider: Arc<Provider<Http>>,
}

impl FlashLoanManager {
    pub fn new(provider: Arc<Provider<Http>>) -> Self {
        let providers = vec![
            FlashLoanProvider {
                name: "Aave V3".to_string(),
                address: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2".to_string(),
                fee_percentage: Decimal::from_str("0.0009").unwrap(), // 0.09%
                supported_tokens: vec![
                    "USDC".to_string(),
                    "USDT".to_string(),
                    "DAI".to_string(),
                    "WETH".to_string(),
                ],
                max_loan_amount: Decimal::from(1_000_000_000),
            },
            FlashLoanProvider {
                name: "Balancer".to_string(),
                address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".to_string(),
                fee_percentage: Decimal::ZERO,
                supported_tokens: vec![
                    "USDC".to_string(),
                    "USDT".to_string(),
                    "DAI".to_string(),
                    "WETH".to_string(),
                    "WBTC".to_string(),
                ],
                max_loan_amount: Decimal::from(500_000_000),
            },
        ];

        Self {
            providers,
            provider,
        }
    }

    pub fn get_cheapest_provider(&self, token: &str, amount: Decimal) -> Option<&FlashLoanProvider> {
        self.providers
            .iter()
            .filter(|p| {
                p.supported_tokens.contains(&token.to_string()) 
                && p.max_loan_amount >= amount
            })
            .min_by_key(|p| {
                // Using to_u64() with proper import
                (p.fee_percentage * Decimal::from(10000))
                    .to_u64()
                    .unwrap_or(u64::MAX)
            })
    }

    pub async fn execute_flash_loan(
        &self,
        _provider: &FlashLoanProvider,
        _token_address: &str,
        _amount: Decimal,
    ) -> Result<()> {
        // Implementation placeholder
        Ok(())
    }
}