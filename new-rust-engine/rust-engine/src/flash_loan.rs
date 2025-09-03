// rust-engine/src/flash_loan.rs
use ethers::prelude::*;
use ethers::types::transaction::eip2718::TypedTransaction;
use ethers::abi::Token;
use std::sync::Arc;
use std::env;
use anyhow::{Result, anyhow};

// Import from config module
use crate::config::ChainConfig;
use crate::scanner::Opportunity;

pub struct FlashLoanExecutor {
    provider: Arc<Provider<Http>>,
    config: ChainConfig,
    wallet: LocalWallet,
}

impl FlashLoanExecutor {
    pub fn new(provider: Arc<Provider<Http>>, config: ChainConfig) -> Self {
        // Get private key from environment variable
        let private_key = env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY must be set in .env file");
        
        // Parse the private key (add 0x if not present)
        let private_key = if private_key.starts_with("0x") {
            private_key
        } else {
            format!("0x{}", private_key)
        };
        
        let wallet = private_key
            .parse::<LocalWallet>()
            .expect("Invalid private key format")
            .with_chain_id(config.chain_id);
        
        Self {
            provider,
            config,
            wallet,
        }
    }
    
    pub async fn execute_opportunity(&self, opp: &Opportunity) -> Result<H256> {
        // Check if we have enough ETH for gas
        let balance = self.provider.get_balance(self.wallet.address(), None).await?;
        let gas_price = self.provider.get_gas_price().await?;
        let estimated_gas_cost = gas_price * U256::from(750_000u64); // Estimated gas units
        
        if balance < estimated_gas_cost {
            return Err(anyhow!("Insufficient ETH for gas. Need at least {} ETH", 
                ethers::utils::format_ether(estimated_gas_cost)));
        }
        
        let flash_provider = self.select_best_provider(opp).await?;
        
        let tx = match flash_provider {
            FlashLoanProvider::AaveV3(addr) => {
                self.build_aave_flash_loan(addr, opp).await?
            },
            FlashLoanProvider::Balancer(addr) => {
                self.build_balancer_flash_loan(addr, opp).await?
            },
            FlashLoanProvider::UniswapV3(addr) => {
                self.build_uniswap_flash_loan(addr, opp).await?
            },
        };
        
        // Apply gas multiplier from environment
        let gas_multiplier: f64 = env::var("GAS_MULTIPLIER")
            .unwrap_or_else(|_| "1.0".to_string())
            .parse()
            .unwrap_or(1.0);
        
        let adjusted_gas_price = U256::from((gas_price.as_u64() as f64 * gas_multiplier) as u64);
        
        // Create a mutable transaction to set gas price
        let mut tx_with_gas = tx;
        tx_with_gas.set_gas_price(adjusted_gas_price);
        
        let client = SignerMiddleware::new(self.provider.clone(), self.wallet.clone());
        let pending = client
            .send_transaction(tx_with_gas, None)
            .await?;
        
        Ok(pending.tx_hash())
    }
    
    async fn build_aave_flash_loan(&self, pool: Address, opp: &Opportunity) -> Result<TypedTransaction> {
        let abi = ethers::abi::parse_abi(&[
            "function flashLoanSimple(address receiverAddress, address asset, uint256 amount, bytes params, uint16 referralCode) returns ()"
        ])?;
        
        let contract = Contract::new(pool, abi, self.provider.clone());
        
        let params = ethers::abi::encode(&[
            Token::Address(opp.pair1),
            Token::Address(opp.pair2),
            Token::Uint(opp.optimal_amount),
        ]);
        
        let tx = contract
            .method::<_, ()>(
                "flashLoanSimple",
                (
                    self.wallet.address(),
                    opp.token0,
                    opp.optimal_amount,
                    Bytes::from(params),
                    0u16,
                ),
            )?
            .tx;
        
        Ok(tx)
    }
    
    async fn build_balancer_flash_loan(&self, vault: Address, opp: &Opportunity) -> Result<TypedTransaction> {
        let abi = ethers::abi::parse_abi(&[
            "function flashLoan(address recipient, address[] tokens, uint256[] amounts, bytes userData) returns ()"
        ])?;
        
        let contract = Contract::new(vault, abi, self.provider.clone());
        
        let user_data = ethers::abi::encode(&[
            Token::Address(opp.pair1),
            Token::Address(opp.pair2),
            Token::Uint(opp.optimal_amount),
        ]);
        
        let tx = contract
            .method::<_, ()>(
                "flashLoan",
                (
                    self.wallet.address(),
                    vec![opp.token0],
                    vec![opp.optimal_amount],
                    Bytes::from(user_data),
                ),
            )?
            .tx;
        
        Ok(tx)
    }
    
    async fn build_uniswap_flash_loan(&self, pool: Address, opp: &Opportunity) -> Result<TypedTransaction> {
        let abi = ethers::abi::parse_abi(&[
            "function flash(address recipient, uint256 amount0, uint256 amount1, bytes data) returns ()"
        ])?;
        
        let contract = Contract::new(pool, abi, self.provider.clone());
        
        let (amount0, amount1) = if self.is_token0(pool, opp.token0).await? {
            (opp.optimal_amount, U256::zero())
        } else {
            (U256::zero(), opp.optimal_amount)
        };
        
        let data = ethers::abi::encode(&[
            Token::Address(opp.pair1),
            Token::Address(opp.pair2),
            Token::Address(opp.token0),
            Token::Address(opp.token1),
        ]);
        
        let tx = contract
            .method::<_, ()>(
                "flash",
                (
                    self.wallet.address(),
                    amount0,
                    amount1,
                    Bytes::from(data),
                ),
            )?
            .tx;
        
        Ok(tx)
    }
    
    async fn select_best_provider(&self, _opp: &Opportunity) -> Result<FlashLoanProvider> {
        match self.config.chain_id {
            42161 | 10 => Ok(FlashLoanProvider::AaveV3(self.config.flash_loan_providers[0])),
            8453 => Ok(FlashLoanProvider::Balancer(self.config.flash_loan_providers[0])),
            _ => Ok(FlashLoanProvider::AaveV3(self.config.flash_loan_providers[0])),
        }
    }
    
    async fn is_token0(&self, pool: Address, token: Address) -> Result<bool> {
        let abi = ethers::abi::parse_abi(&[
            "function token0() view returns (address)"
        ])?;
        
        let contract = Contract::new(pool, abi, self.provider.clone());
        let token0: Address = contract.method("token0", ())?.call().await?;
        
        Ok(token0 == token)
    }
}

#[derive(Debug)]
enum FlashLoanProvider {
    AaveV3(Address),
    Balancer(Address),
    UniswapV3(Address),
}