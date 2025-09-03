use ethers::prelude::*;
use ethers::signers::{LocalWallet, Signer};
use std::sync::Arc;
use anyhow::Result;

pub struct WalletManager {
    wallet: LocalWallet,
    provider: Arc<Provider<Http>>,
    client: Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>>,
}

impl WalletManager {
    pub fn new() -> Result<Self> {
        let private_key = std::env::var("PRIVATE_KEY")
            .expect("PRIVATE_KEY must be set in .env file");
        
        let wallet = private_key.parse::<LocalWallet>()?
            .with_chain_id(42161u64); // Arbitrum One
        
        let provider = Provider::<Http>::try_from(
            std::env::var("RPC_URL").unwrap_or_else(|_| 
                "https://arb1.arbitrum.io/rpc".to_string())
        )?;
        
        let provider = Arc::new(provider);
        let client = Arc::new(SignerMiddleware::new(
            provider.clone(), 
            wallet.clone()
        ));
        
        Ok(Self {
            wallet,
            provider,
            client,
        })
    }
    
    pub fn address(&self) -> Address {
        self.wallet.address()
    }
    
    pub async fn get_balance(&self) -> Result<U256> {
        Ok(self.provider.get_balance(self.wallet.address(), None).await?)
    }
    
    pub fn client(&self) -> Arc<SignerMiddleware<Arc<Provider<Http>>, LocalWallet>> {
        self.client.clone()
    }
}