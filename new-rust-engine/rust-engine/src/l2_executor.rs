// File: src/l2_executor.rs

use ethers::prelude::*;
use ethers::utils::parse_ether;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

pub struct L2ExecutionEngine {
    providers: std::collections::HashMap<String, Arc<SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>>>,
    flashloan_contracts: std::collections::HashMap<String, Address>,
    max_slippage_bps: u32,
    max_gas_price_gwei: u64,
}

impl L2ExecutionEngine {
    pub async fn new(private_key: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let wallet = private_key.parse::<LocalWallet>()?;
        let mut providers = std::collections::HashMap::new();
        let mut flashloan_contracts = std::collections::HashMap::new();
        
        // Arbitrum setup
        let arb_provider = Provider::<Http>::try_from("https://arb1.arbitrum.io/rpc")?;
        let arb_wallet = wallet.clone().with_chain_id(42161u64);
        let arb_client = Arc::new(SignerMiddleware::new(arb_provider, arb_wallet));
        providers.insert("arbitrum".to_string(), arb_client);
        
        // Balancer Vault for flash loans on Arbitrum
        flashloan_contracts.insert(
            "arbitrum".to_string(),
            "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?
        );
        
        // Optimism setup
        let opt_provider = Provider::<Http>::try_from("https://mainnet.optimism.io")?;
        let opt_wallet = wallet.clone().with_chain_id(10u64);
        let opt_client = Arc::new(SignerMiddleware::new(opt_provider, opt_wallet));
        providers.insert("optimism".to_string(), opt_client);
        
        flashloan_contracts.insert(
            "optimism".to_string(),
            "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?
        );
        
        // Base setup
        let base_provider = Provider::<Http>::try_from("https://mainnet.base.org")?;
        let base_wallet = wallet.clone().with_chain_id(8453u64);
        let base_client = Arc::new(SignerMiddleware::new(base_provider, base_wallet));
        providers.insert("base".to_string(), base_client);
        
        flashloan_contracts.insert(
            "base".to_string(),
            "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse()?
        );
        
        Ok(Self {
            providers,
            flashloan_contracts,
            max_slippage_bps: 100, // 1% max slippage
            max_gas_price_gwei: 50, // Max 50 gwei gas price
        })
    }
    
    pub async fn execute_arbitrage(
        &self,
        network: &str,
        buy_dex: Address,
        sell_dex: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
        min_profit: U256,
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error>> {
        let provider = self.providers.get(network)
            .ok_or("Network not supported")?;
        
        // Check current gas price
        let gas_price = provider.get_gas_price().await?;
        if gas_price > U256::from(self.max_gas_price_gwei) * U256::from(1_000_000_000u64) {
            return Err("Gas price too high".into());
        }
        
        // Simulate transaction first
        let simulated_profit = self.simulate_arbitrage(
            network,
            buy_dex,
            sell_dex,
            token_in,
            token_out,
            amount_in
        ).await?;
        
        if simulated_profit < min_profit {
            return Err("Simulated profit too low".into());
        }
        
        // Build arbitrage contract call
        let arb_contract = self.deploy_arbitrage_contract(network).await?;
        
        let arb_abi = ethers::abi::parse_abi(&[
            "function executeArbitrage(address,address,address,address,uint256,uint256)",
        ])?;
        
        let contract = Contract::new(arb_contract, arb_abi, provider.clone());
        
        // Execute with flash loan
        let tx = contract
            .method::<_, H256>(
                "executeArbitrage",
                (buy_dex, sell_dex, token_in, token_out, amount_in, min_profit)
            )?
            .gas(500000)
            .gas_price(gas_price)
            .send()
            .await?;
        
        // Wait for confirmation with timeout
        let receipt = timeout(
            Duration::from_secs(30),
            tx
        ).await??
            .ok_or("Transaction failed")?;
        
        Ok(receipt)
    }
    
    async fn simulate_arbitrage(
        &self,
        network: &str,
        buy_dex: Address,
        sell_dex: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        let provider = self.providers.get(network)
            .ok_or("Network not supported")?;
        
        // Get output amount from buy DEX
        let buy_output = self.get_amount_out(
            provider.clone(),
            buy_dex,
            token_in,
            token_out,
            amount_in
        ).await?;
        
        // Get output amount from selling on sell DEX
        let sell_output = self.get_amount_out(
            provider.clone(),
            sell_dex,
            token_out,
            token_in,
            buy_output
        ).await?;
        
        // Calculate profit
        if sell_output > amount_in {
            Ok(sell_output - amount_in)
        } else {
            Ok(U256::zero())
        }
    }
    
    async fn get_amount_out(
        &self,
        provider: Arc<SignerMiddleware<Provider<Http>, Wallet<k256::ecdsa::SigningKey>>>,
        router: Address,
        token_in: Address,
        token_out: Address,
        amount_in: U256,
    ) -> Result<U256, Box<dyn std::error::Error>> {
        let router_abi = ethers::abi::parse_abi(&[
            "function getAmountsOut(uint256,address[]) view returns (uint256[])",
        ])?;
        
        let contract = Contract::new(router, router_abi, provider);
        
        let path = vec![token_in, token_out];
        let amounts: Vec<U256> = contract
            .method("getAmountsOut", (amount_in, path))?
            .call()
            .await?;
        
        Ok(amounts[1])
    }
    
    async fn deploy_arbitrage_contract(
        &self,
        network: &str
    ) -> Result<Address, Box<dyn std::error::Error>> {
        // Return pre-deployed arbitrage contract address for each network
        match network {
            "arbitrum" => Ok("0x0000000000000000000000000000000000000001".parse()?), // Deploy actual contract
            "optimism" => Ok("0x0000000000000000000000000000000000000002".parse()?),
            "base" => Ok("0x0000000000000000000000000000000000000003".parse()?),
            _ => Err("Network not supported".into())
        }
    }
    
    pub async fn check_balances(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("💰 Checking wallet balances across L2 networks...\n");
        
        for (network, provider) in &self.providers {
            let address = provider.address();
            let balance = provider.get_balance(address, None).await?;
            let eth_balance = ethers::utils::format_ether(balance);
            
            println!("  {} Balance:", network);
            println!("    Address: {}", address);
            println!("    ETH: {} ETH", eth_balance);
            
            if balance < U256::from(10_000_000_000_000_000u64) {
                println!("    ⚠️  Low balance! Need at least 0.01 ETH for gas");
            }
            println!();
        }
        
        Ok(())
    }
}