use crate::{
    config::Config,
    types::*,
    arbitrage::FlashLoanInfo,
};
use anyhow::Result;
use rust_decimal::Decimal;
use std::sync::Arc;
use ethers::{
    prelude::*,
    providers::{Provider, Http},
};
use std::str::FromStr;

abigen!(
    AavePool,
    r#"[
        function flashLoan(address receiverAddress, address[] calldata assets, uint256[] calldata amounts, uint256[] calldata modes, address onBehalfOf, bytes calldata params, uint16 referralCode) external
        function FLASHLOAN_PREMIUM_TOTAL() external view returns (uint128)
        function getReserveData(address asset) external view returns (uint256 configuration, uint128 liquidityIndex, uint128 variableBorrowIndex, uint128 currentLiquidityRate, uint128 currentVariableBorrowRate, uint128 currentStableBorrowRate, uint40 lastUpdateTimestamp, address aTokenAddress, address stableDebtTokenAddress, address variableDebtTokenAddress, address interestRateStrategyAddress, uint8 id)
    ]"#
);

abigen!(
    BalancerVault,
    r#"[
        function flashLoan(address recipient, address[] memory tokens, uint256[] memory amounts, bytes memory userData) external
        function getProtocolFeesCollector() external view returns (address)
    ]"#
);

abigen!(
    DyDxSoloMargin,
    r#"[
        function operate(AccountInfo[] memory accounts, ActionArgs[] memory actions) external
        function getMarketTokenAddress(uint256 marketId) external view returns (address)
        function getNumMarkets() external view returns (uint256)
    ]"#
);

pub struct FlashLoanCalculator {
    providers: Vec<Arc<FlashLoanProvider>>,
    config: Config,
}

impl FlashLoanCalculator {
    pub async fn new(config: Config) -> Result<Arc<Self>> {
        let mut providers = Vec::new();

        for flash_config in &config.flash_loan_providers {
            if flash_config.enabled {
                let provider = FlashLoanProvider::new(flash_config.clone()).await?;
                providers.push(Arc::new(provider));
            }
        }

        Ok(Arc::new(Self {
            providers,
            config,
        }))
    }

    pub async fn calculate_best_loan(
        &self,
        amount: Decimal,
        chain_id: u64,
    ) -> Result<FlashLoanInfo> {
        let mut best_loan: Option<FlashLoanInfo> = None;
        let mut lowest_fee = Decimal::MAX;

        for provider in &self.providers {
            if provider.chain_id != chain_id {
                continue;
            }

            let fee = provider.calculate_fee(amount).await?;
            
            if fee < lowest_fee {
                lowest_fee = fee;
                best_loan = Some(FlashLoanInfo {
                    provider: provider.name.clone(),
                    fee,
                    fee_percentage: provider.fee_percentage,
                    max_amount: provider.get_max_loan_amount().await?,
                });
            }
        }

        best_loan.ok_or_else(|| anyhow::anyhow!("No flash loan provider available"))
    }

    pub async fn get_available_providers(&self, chain_id: u64) -> Vec<String> {
        self.providers
            .iter()
            .filter(|p| p.chain_id == chain_id)
            .map(|p| p.name.clone())
            .collect()
    }

    pub async fn estimate_total_cost(
        &self,
        provider_name: &str,
        amount: Decimal,
        gas_price: Decimal,
    ) -> Result<Decimal> {
        let provider = self.providers
            .iter()
            .find(|p| p.name == provider_name)
            .ok_or_else(|| anyhow::anyhow!("Provider not found"))?;

        let loan_fee = provider.calculate_fee(amount).await?;
        let gas_cost = self.estimate_gas_cost(provider_name, gas_price).await?;

        Ok(loan_fee + gas_cost)
    }

    async fn estimate_gas_cost(
        &self,
        provider_name: &str,
        gas_price: Decimal,
    ) -> Result<Decimal> {
        let gas_units = match provider_name {
            "Aave V3" => 300000u64,
            "Balancer" => 250000u64,
            "dYdX" => 350000u64,
            _ => 300000u64,
        };

        let eth_price = Decimal::from(2000);
        let gas_cost_eth = Decimal::from(gas_units) * gas_price / Decimal::from(1_000_000_000);
        let gas_cost_usd = gas_cost_eth * eth_price;

        Ok(gas_cost_usd)
    }
}

struct FlashLoanProvider {
    name: String,
    chain_id: u64,
    contract_address: Address,
    fee_percentage: Decimal,
    provider: Arc<Provider<Http>>,
}

impl FlashLoanProvider {
    async fn new(config: crate::config::FlashLoanConfig) -> Result<Self> {
        let chain_config = crate::config::ChainConfig {
            chain_id: config.chain_id,
            name: format!("Chain-{}", config.chain_id),
            rpc_urls: vec!["https://eth.llamarpc.com".to_string()],
            ws_url: None,
            native_token: "ETH".to_string(),
            explorer_api_key: None,
            enabled: true,
        };

        let provider = Provider::<Http>::try_from(&chain_config.rpc_urls[0])?;
        let provider = Arc::new(provider);

        Ok(Self {
            name: config.provider,
            chain_id: config.chain_id,
            contract_address: Address::from_str(&config.pool_address)?,
            fee_percentage: config.fee_percentage,
            provider,
        })
    }

    async fn calculate_fee(&self, amount: Decimal) -> Result<Decimal> {
        Ok(amount * self.fee_percentage)
    }

    async fn get_max_loan_amount(&self) -> Result<Decimal> {
        match self.name.as_str() {
            "Aave V3" => {
                Ok(Decimal::from(100_000_000))
            },
            "Balancer" => {
                Ok(Decimal::from(50_000_000))
            },
            "dYdX" => {
                Ok(Decimal::from(10_000_000))
            },
            _ => Ok(Decimal::from(1_000_000)),
        }
    }

    async fn check_liquidity(&self, token_address: Address) -> Result<Decimal> {
        match self.name.as_str() {
            "Aave V3" => {
                let aave = AavePool::new(self.contract_address, self.provider.clone());
                
                match aave.get_reserve_data(token_address).call().await {
                    Ok(data) => {
                        let liquidity = U256::from(data.3);
                        Ok(Decimal::from_str(&liquidity.to_string())?)
                    },
                    Err(_) => Ok(Decimal::from(1_000_000)),
                }
            },
            _ => Ok(Decimal::from(1_000_000)),
        }
    }
}