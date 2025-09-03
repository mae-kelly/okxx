use ethers::prelude::*;
use ethers::core::types::transaction::eip2718::TypedTransaction;
use anyhow::Result;
use std::sync::Arc;
use crate::wallet::WalletManager;
use crate::config::Config;

#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub token_a: Address,
    pub token_b: Address,
    pub buy_from_dex: String,
    pub sell_to_dex: String,
    pub optimal_amount: U256,
    pub estimated_profit: U256,
    pub profit_after_gas: U256,
    pub gas_estimate: U256,
}

pub struct ArbitrageExecutor {
    wallet: WalletManager,
    provider: Arc<Provider<Ws>>,
    config: Config,
    flashloan_contract: Address,
}

impl ArbitrageExecutor {
    pub fn new(wallet: WalletManager, provider: Arc<Provider<Ws>>, config: Config) -> Self {
        let flashloan_contract = std::env::var("FLASHLOAN_CONTRACT")
            .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000".to_string())
            .parse::<Address>()
            .expect("Invalid flashloan contract address");
        
        Self {
            wallet,
            provider,
            config,
            flashloan_contract,
        }
    }
    
    pub async fn execute_with_flashloan(&self, opp: ArbitrageOpportunity) -> Result<H256> {
        // Build flashloan execution data
        let abi = ethers::abi::parse_abi(&[
            "function executeFlashLoan(address asset, uint256 amount, bytes calldata params)"
        ])?;
        
        let contract = Contract::new(
            self.flashloan_contract,
            abi,
            self.wallet.client()
        );
        
        // Encode swap parameters
        let swap_params = ethers::abi::encode(&[
            ethers::abi::Token::Address(opp.token_a),
            ethers::abi::Token::Address(opp.token_b),
            ethers::abi::Token::Uint(opp.optimal_amount),
            ethers::abi::Token::String(opp.buy_from_dex),
            ethers::abi::Token::String(opp.sell_to_dex),
        ]);
        
        // Execute with higher gas price for priority
        let gas_price = self.provider.get_gas_price().await?;
        let priority_gas = gas_price * 120 / 100; // 20% higher
        
        // Fix: Create the call and store it in a variable
        let call = contract
            .method::<_, ()>("executeFlashLoan", (opp.token_a, opp.optimal_amount, swap_params))?
            .gas(500000)
            .gas_price(priority_gas);
        
        let pending_tx = call.send().await?;
        let tx_hash = pending_tx.tx_hash();
        
        Ok(tx_hash)
    }
    
    pub async fn simulate_arbitrage(&self, opp: &ArbitrageOpportunity) -> Result<bool> {
        // Simulate the transaction before executing
        // This prevents wasting gas on failed transactions
        
        let call_data = self.build_arbitrage_calldata(opp)?;
        
        let tx = TypedTransaction::Legacy(TransactionRequest {
            to: Some(NameOrAddress::Address(self.flashloan_contract)),
            data: Some(call_data),
            gas: Some(U256::from(500000)),
            ..Default::default()
        });
        
        match self.provider.call(&tx, None).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    fn build_arbitrage_calldata(&self, opp: &ArbitrageOpportunity) -> Result<Bytes> {
        // Build the calldata for the arbitrage execution
        let abi = ethers::abi::parse_abi(&[
            "function executeFlashLoan(address,uint256,bytes)"
        ])?;
        
        let func = abi.function("executeFlashLoan")?;
        let params = ethers::abi::encode(&[
            ethers::abi::Token::Address(opp.token_a),
            ethers::abi::Token::Uint(opp.optimal_amount),
            ethers::abi::Token::Bytes(vec![]),
        ]);
        
        Ok(func.encode_input(&[
            ethers::abi::Token::Address(opp.token_a),
            ethers::abi::Token::Uint(opp.optimal_amount),
            ethers::abi::Token::Bytes(params),
        ])?.into())
    }
}