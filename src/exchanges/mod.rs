pub mod binance;
pub mod coinbase;
pub mod kraken;
pub mod uniswap;
pub mod sushiswap;
pub mod pancakeswap;
pub mod curve;
pub mod balancer;

use crate::{config::Config, types::*};
use anyhow::Result;
use async_trait::async_trait;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use rust_decimal::Decimal;

#[async_trait]
pub trait Exchange: Send + Sync {
    async fn get_name(&self) -> String;
    async fn get_pairs(&self) -> Result<Vec<TokenPair>>;
    async fn get_price(&self, pair: &TokenPair) -> Result<Price>;
    async fn get_orderbook(&self, pair: &TokenPair, depth: usize) -> Result<OrderBook>;
    async fn get_fees(&self) -> Result<ExchangeFees>;
    async fn get_24h_volume(&self, pair: &TokenPair) -> Result<Decimal>;
    async fn subscribe_to_updates(&self, pairs: Vec<TokenPair>) -> Result<()>;
}

pub struct ExchangeManager {
    exchanges: Vec<Arc<dyn Exchange>>,
    price_cache: Arc<DashMap<String, Price>>,
    config: Config,
}

impl ExchangeManager {
    pub async fn new(
        config: Config,
        price_cache: Arc<DashMap<String, Price>>,
    ) -> Result<Arc<Self>> {
        let mut exchanges: Vec<Arc<dyn Exchange>> = Vec::new();

        if config.exchanges.binance.enabled {
            exchanges.push(Arc::new(
                binance::BinanceExchange::new(config.exchanges.binance.clone()).await?
            ));
        }

        if config.exchanges.coinbase.enabled {
            exchanges.push(Arc::new(
                coinbase::CoinbaseExchange::new(config.exchanges.coinbase.clone()).await?
            ));
        }

        if config.exchanges.kraken.enabled {
            exchanges.push(Arc::new(
                kraken::KrakenExchange::new(config.exchanges.kraken.clone()).await?
            ));
        }

        if config.exchanges.uniswap_v3.enabled {
            for chain_config in &config.chains {
                if let Some(router) = config.exchanges.uniswap_v3.router_address.get(&chain_config.chain_id) {
                    exchanges.push(Arc::new(
                        uniswap::UniswapV3Exchange::new(
                            chain_config.clone(),
                            router.clone(),
                            config.exchanges.uniswap_v3.factory_address
                                .get(&chain_config.chain_id)
                                .cloned()
                                .unwrap_or_default(),
                        ).await?
                    ));
                }
            }
        }

        if config.exchanges.sushiswap.enabled {
            for chain_config in &config.chains {
                if let Some(router) = config.exchanges.sushiswap.router_address.get(&chain_config.chain_id) {
                    exchanges.push(Arc::new(
                        sushiswap::SushiswapExchange::new(
                            chain_config.clone(),
                            router.clone(),
                            config.exchanges.sushiswap.factory_address
                                .get(&chain_config.chain_id)
                                .cloned()
                                .unwrap_or_default(),
                        ).await?
                    ));
                }
            }
        }

        if config.exchanges.pancakeswap.enabled {
            for chain_config in &config.chains {
                if let Some(router) = config.exchanges.pancakeswap.router_address.get(&chain_config.chain_id) {
                    exchanges.push(Arc::new(
                        pancakeswap::PancakeswapExchange::new(
                            chain_config.clone(),
                            router.clone(),
                            config.exchanges.pancakeswap.factory_address
                                .get(&chain_config.chain_id)
                                .cloned()
                                .unwrap_or_default(),
                        ).await?
                    ));
                }
            }
        }

        if config.exchanges.curve.enabled {
            for chain_config in &config.chains {
                if let Some(router) = config.exchanges.curve.router_address.get(&chain_config.chain_id) {
                    exchanges.push(Arc::new(
                        curve::CurveExchange::new(
                            chain_config.clone(),
                            router.clone(),
                        ).await?
                    ));
                }
            }
        }

        if config.exchanges.balancer.enabled {
            for chain_config in &config.chains {
                if let Some(vault) = config.exchanges.balancer.router_address.get(&chain_config.chain_id) {
                    exchanges.push(Arc::new(
                        balancer::BalancerExchange::new(
                            chain_config.clone(),
                            vault.clone(),
                        ).await?
                    ));
                }
            }
        }

        Ok(Arc::new(Self {
            exchanges,
            price_cache,
            config,
        }))
    }

    pub async fn get_all_pairs(&self) -> Result<Vec<TokenPair>> {
        let mut all_pairs = Vec::new();
        
        for exchange in &self.exchanges {
            match exchange.get_pairs().await {
                Ok(pairs) => all_pairs.extend(pairs),
                Err(e) => {
                    tracing::warn!("Failed to get pairs from {}: {}", 
                        exchange.get_name().await, e);
                }
            }
        }

        all_pairs.sort_by(|a, b| {
            format!("{}-{}", a.base.symbol, a.quote.symbol)
                .cmp(&format!("{}-{}", b.base.symbol, b.quote.symbol))
        });
        all_pairs.dedup_by(|a, b| {
            a.base.symbol == b.base.symbol && a.quote.symbol == b.quote.symbol
        });

        Ok(all_pairs)
    }

    pub async fn get_prices_for_pair(&self, pair: &TokenPair) -> Result<Vec<Price>> {
        let mut prices = Vec::new();
        
        for exchange in &self.exchanges {
            match exchange.get_price(pair).await {
                Ok(price) => {
                    let cache_key = format!("{}-{}-{}", 
                        exchange.get_name().await,
                        pair.base.symbol,
                        pair.quote.symbol
                    );
                    self.price_cache.insert(cache_key, price.clone());
                    prices.push(price);
                },
                Err(e) => {
                    tracing::debug!("Failed to get price from {}: {}", 
                        exchange.get_name().await, e);
                }
            }
        }

        Ok(prices)
    }

    pub async fn get_orderbook(&self, exchange_name: &str, pair: &TokenPair, depth: usize) -> Result<OrderBook> {
        for exchange in &self.exchanges {
            if exchange.get_name().await == exchange_name {
                return exchange.get_orderbook(pair, depth).await;
            }
        }
        
        Err(anyhow::anyhow!("Exchange {} not found", exchange_name))
    }

    pub async fn get_exchange_fees(&self, exchange_name: &str) -> Result<ExchangeFees> {
        for exchange in &self.exchanges {
            if exchange.get_name().await == exchange_name {
                return exchange.get_fees().await;
            }
        }
        
        Err(anyhow::anyhow!("Exchange {} not found", exchange_name))
    }

    pub async fn get_24h_volume(&self, exchange_name: &str, pair: &TokenPair) -> Result<Decimal> {
        for exchange in &self.exchanges {
            if exchange.get_name().await == exchange_name {
                return exchange.get_24h_volume(pair).await;
            }
        }
        
        Err(anyhow::anyhow!("Exchange {} not found", exchange_name))
    }

    pub fn get_cached_price(&self, exchange: &str, pair: &TokenPair) -> Option<Price> {
        let key = format!("{}-{}-{}", exchange, pair.base.symbol, pair.quote.symbol);
        self.price_cache.get(&key).map(|entry| entry.clone())
    }

    pub async fn subscribe_all(&self, pairs: Vec<TokenPair>) -> Result<()> {
        for exchange in &self.exchanges {
            if let Err(e) = exchange.subscribe_to_updates(pairs.clone()).await {
                tracing::warn!("Failed to subscribe on {}: {}", 
                    exchange.get_name().await, e);
            }
        }
        Ok(())
    }
}pub mod okx;
