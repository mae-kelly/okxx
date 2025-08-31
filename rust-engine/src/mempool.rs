use ethers::prelude::*;
use std::sync::Arc;
use tokio_stream::StreamExt;

pub struct MempoolMonitor {
    provider: Arc<Provider<Ws>>,
}

impl MempoolMonitor {
    pub fn new(provider: Arc<Provider<Ws>>) -> Self {
        Self { provider }
    }
    
    pub async fn watch_pending_txs<F>(&self, mut handler: F) 
    where
        F: FnMut(Transaction) + Send + 'static
    {
        let mut stream = self.provider.watch_pending_transactions().await.unwrap();
        
        while let Some(tx_hash) = stream.next().await {
            if let Ok(Some(tx)) = self.provider.get_transaction(tx_hash).await {
                // Decode and analyze transaction
                if Self::is_dex_trade(&tx) {
                    handler(tx);
                }
            }
        }
    }
    
    fn is_dex_trade(tx: &Transaction) -> bool {
        // Check if transaction is interacting with known DEX routers
        const UNISWAP_V2: Address = Address::from_slice(&hex!("7a250d5630B4cF539739dF2C5dAcb4c659F2488D"));
        const UNISWAP_V3: Address = Address::from_slice(&hex!("E592427A0AEce92De3Edee1F18E0157C05861564"));
        
        tx.to == Some(UNISWAP_V2) || tx.to == Some(UNISWAP_V3)
    }
}