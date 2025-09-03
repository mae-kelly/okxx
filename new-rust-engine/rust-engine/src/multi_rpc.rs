// rust-engine/src/multi_rpc.rs
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use ethers::prelude::*;
use anyhow::Result;

/// Rotates between multiple RPC endpoints to avoid rate limits
pub struct MultiRpcProvider {
    providers: Vec<Arc<Provider<Http>>>,
    current: AtomicUsize,
}

impl MultiRpcProvider {
    pub fn new(rpc_urls: Vec<String>) -> Result<Self> {
        let mut providers = Vec::new();
        
        for url in rpc_urls {
            let provider = Provider::<Http>::try_from(url)?;
            providers.push(Arc::new(provider));
        }
        
        Ok(Self {
            providers,
            current: AtomicUsize::new(0),
        })
    }
    
    /// Get next provider in rotation
    pub fn get_provider(&self) -> Arc<Provider<Http>> {
        let idx = self.current.fetch_add(1, Ordering::Relaxed) % self.providers.len();
        self.providers[idx].clone()
    }
    
    /// Execute with automatic retry on different RPCs
    pub async fn execute_with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn(Arc<Provider<Http>>) -> futures::future::BoxFuture<'static, Result<T>>,
    {
        let mut last_error = None;
        
        // Try each provider
        for _ in 0..self.providers.len() {
            let provider = self.get_provider();
            
            match operation(provider).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    // Continue to next provider
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All RPCs failed")))
    }
}