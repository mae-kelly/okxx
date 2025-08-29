use crate::{
    config::Config,
    types::*,
    exchanges::ExchangeManager,
    arbitrage::ArbitrageDetector,
    data_store::DataStore,
    metrics::MetricsCollector,
};
use anyhow::Result;
use std::sync::Arc;
use tokio::time::{interval, Duration};
use futures::future::join_all;
use tracing::{info, warn, error};
use dashmap::DashMap;
use chrono::Utc;

pub struct Scanner {
    config: Config,
    exchange_manager: Arc<ExchangeManager>,
    arbitrage_detector: Arc<ArbitrageDetector>,
    data_store: Arc<DataStore>,
    metrics: Arc<MetricsCollector>,
    active_pairs: Arc<DashMap<String, TokenPair>>,
}

impl Scanner {
    pub fn new(
        config: Config,
        exchange_manager: Arc<ExchangeManager>,
        arbitrage_detector: Arc<ArbitrageDetector>,
        data_store: Arc<DataStore>,
        metrics: Arc<MetricsCollector>,
    ) -> Self {
        Self {
            config,
            exchange_manager,
            arbitrage_detector,
            data_store,
            metrics,
            active_pairs: Arc::new(DashMap::new()),
        }
    }

    pub async fn run(&self) -> Result<()> {
        self.initialize_pairs().await?;
        
        let mut scan_interval = interval(Duration::from_millis(self.config.scanner.scan_interval_ms));
        
        loop {
            scan_interval.tick().await;
            
            let start_time = Utc::now();
            
            if let Err(e) = self.scan_cycle().await {
                error!("Scan cycle error: {}", e);
                self.metrics.record_error("scan_cycle");
            }
            
            let duration = (Utc::now() - start_time).num_milliseconds() as u64;
            self.metrics.record_scan_duration(duration);
        }
    }

    async fn initialize_pairs(&self) -> Result<()> {
        info!("Initializing trading pairs");
        
        let pairs = self.exchange_manager.get_all_pairs().await?;
        
        for pair in pairs {
            let key = format!("{}-{}", pair.base.symbol, pair.quote.symbol);
            self.active_pairs.insert(key, pair);
        }
        
        info!("Initialized {} trading pairs", self.active_pairs.len());
        Ok(())
    }

    async fn scan_cycle(&self) -> Result<()> {
        let pairs: Vec<TokenPair> = self.active_pairs
            .iter()
            .map(|entry| entry.value().clone())
            .collect();

        let chunk_size = pairs.len() / self.config.scanner.concurrent_scans;
        let chunks: Vec<Vec<TokenPair>> = pairs
            .chunks(chunk_size.max(1))
            .map(|chunk| chunk.to_vec())
            .collect();

        let scan_tasks = chunks.into_iter().map(|chunk| {
            let exchange_manager = self.exchange_manager.clone();
            let arbitrage_detector = self.arbitrage_detector.clone();
            let data_store = self.data_store.clone();
            let metrics = self.metrics.clone();
            
            tokio::spawn(async move {
                for pair in chunk {
                    if let Err(e) = Self::scan_pair(
                        &pair,
                        &exchange_manager,
                        &arbitrage_detector,
                        &data_store,
                        &metrics,
                    ).await {
                        warn!("Failed to scan pair {}-{}: {}", 
                            pair.base.symbol, pair.quote.symbol, e);
                    }
                }
            })
        });

        join_all(scan_tasks).await;
        
        Ok(())
    }

    async fn scan_pair(
        pair: &TokenPair,
        exchange_manager: &Arc<ExchangeManager>,
        arbitrage_detector: &Arc<ArbitrageDetector>,
        data_store: &Arc<DataStore>,
        metrics: &Arc<MetricsCollector>,
    ) -> Result<()> {
        let prices = exchange_manager.get_prices_for_pair(pair).await?;
        
        if prices.len() < 2 {
            return Ok(());
        }

        let opportunities = arbitrage_detector.detect_opportunities(&prices, pair).await?;
        
        for opportunity in opportunities {
            metrics.record_opportunity(&opportunity);
            data_store.store_opportunity(&opportunity).await?;
            
            if opportunity.net_profit > rust_decimal::Decimal::ZERO {
                info!(
                    "Found profitable opportunity: {} -> {} for {}-{}, profit: ${}, confidence: {:.2}%",
                    opportunity.buy_exchange,
                    opportunity.sell_exchange,
                    pair.base.symbol,
                    pair.quote.symbol,
                    opportunity.net_profit,
                    opportunity.ml_confidence * 100.0
                );
            }
        }
        
        Ok(())
    }

    pub async fn update_active_pairs(&self) -> Result<()> {
        let new_pairs = self.exchange_manager.get_all_pairs().await?;
        
        self.active_pairs.clear();
        
        for pair in new_pairs {
            let key = format!("{}-{}", pair.base.symbol, pair.quote.symbol);
            self.active_pairs.insert(key, pair);
        }
        
        Ok(())
    }

    pub async fn get_statistics(&self) -> Result<ScannerStatistics> {
        let total_pairs = self.active_pairs.len();
        let opportunities_24h = self.data_store.count_opportunities_24h().await?;
        let profitable_opportunities_24h = self.data_store.count_profitable_opportunities_24h().await?;
        let total_profit_24h = self.data_store.get_total_profit_24h().await?;
        
        Ok(ScannerStatistics {
            total_pairs,
            opportunities_24h,
            profitable_opportunities_24h,
            total_profit_24h,
            last_scan_time: Utc::now(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ScannerStatistics {
    pub total_pairs: usize,
    pub opportunities_24h: u64,
    pub profitable_opportunities_24h: u64,
    pub total_profit_24h: rust_decimal::Decimal,
    pub last_scan_time: chrono::DateTime<Utc>,
}