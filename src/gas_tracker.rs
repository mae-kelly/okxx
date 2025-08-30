use std::sync::Arc;
use anyhow::Result;
use dashmap::DashMap;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal::prelude::FromStr;
use chrono::{DateTime, Utc};
use crate::types::{Chain, GasPrice};

pub struct GasTracker {
    pub gas_history: Arc<DashMap<Chain, Vec<GasPrice>>>,
    update_interval_ms: u64,
}

impl GasTracker {
    pub fn new() -> Self {
        Self {
            gas_history: Arc::new(DashMap::new()),
            update_interval_ms: 5000,
        }
    }
    
    pub async fn update_gas_price(&self, chain: Chain, gas_price: GasPrice) -> Result<()> {
        // Clone chain for the second use
        let chain_clone = chain.clone();
        
        self.gas_history
            .entry(chain)
            .or_insert_with(Vec::new)
            .push(gas_price.clone());
        
        // Limit history to last 100 entries
        if let Some(mut history) = self.gas_history.get_mut(&chain_clone) {
            if history.len() > 100 {
                let drain_count = history.len() - 100;
                history.drain(0..drain_count);
            }
        }
        
        Ok(())
    }
    
    pub fn get_current_gas_price(&self, chain: &Chain) -> Option<GasPrice> {
        self.gas_history.get(chain)
            .and_then(|history| history.last().cloned())
    }
    
    pub fn get_average_gas_price(&self, chain: &Chain, window_minutes: u64) -> Option<Decimal> {
        let history = self.gas_history.get(chain)?;
        let cutoff = Utc::now() - chrono::Duration::minutes(window_minutes as i64);
        
        let recent_prices: Vec<Decimal> = history.iter()
            .filter(|p| p.timestamp > cutoff)
            .map(|p| p.fast)
            .collect();
        
        if recent_prices.is_empty() {
            return None;
        }
        
        let sum: Decimal = recent_prices.iter().sum();
        Some(sum / Decimal::from(recent_prices.len()))
    }
    
    pub fn get_gas_volatility(&self, chain: &Chain) -> Option<Decimal> {
        let history = self.gas_history.get(chain)?;
        
        if history.len() < 2 {
            return None;
        }
        
        let prices: Vec<Decimal> = history.iter()
            .map(|p| p.fast)
            .collect();
        
        let mean = prices.iter().sum::<Decimal>() / Decimal::from(prices.len());
        
        let variance = prices.iter()
            .map(|p| (*p - mean) * (*p - mean))
            .sum::<Decimal>() / Decimal::from(prices.len());
        
        let std_dev = variance.sqrt().unwrap_or(Decimal::ZERO);
        Some(std_dev)
    }
    
    pub fn predict_gas_price(&self, chain: &Chain, minutes_ahead: u64) -> Option<Decimal> {
        let history = self.gas_history.get(chain)?;
        
        if history.len() < 10 {
            return self.get_current_gas_price(chain).map(|p| p.fast);
        }
        
        // Simple linear regression prediction
        let recent: Vec<(i64, Decimal)> = history.iter()
            .rev()
            .take(20)
            .enumerate()
            .map(|(i, p)| (i as i64, p.fast))
            .collect();
        
        let n = recent.len() as i64;
        let sum_x: i64 = recent.iter().map(|(x, _)| x).sum();
        let sum_y: Decimal = recent.iter().map(|(_, y)| *y).sum();
        let sum_xy: Decimal = recent.iter()
            .map(|(x, y)| Decimal::from(*x) * *y)
            .sum();
        let sum_x2: i64 = recent.iter().map(|(x, _)| x * x).sum();
        
        let slope = (Decimal::from(n) * sum_xy - Decimal::from(sum_x) * sum_y) /
                   (Decimal::from(n * sum_x2) - Decimal::from(sum_x * sum_x));
        
        let intercept = (sum_y - slope * Decimal::from(sum_x)) / Decimal::from(n);
        
        let future_x = Decimal::from(minutes_ahead);
        let predicted = intercept + slope * future_x;
        
        Some(predicted.max(Decimal::ZERO))
    }
    
    pub fn get_congestion_level(&self, chain: &Chain) -> String {
        let current = match self.get_current_gas_price(chain) {
            Some(p) => p.fast,
            None => return "Unknown".to_string(),
        };
        
        let avg = match self.get_average_gas_price(chain, 60) {
            Some(a) => a,
            None => return "Unknown".to_string(),
        };
        
        let ratio = current / avg;
        
        if ratio < Decimal::from_str("0.8").unwrap() {
            "Low".to_string()
        } else if ratio < Decimal::from_str("1.2").unwrap() {
            "Normal".to_string()
        } else if ratio < Decimal::from_str("1.5").unwrap() {
            "High".to_string()
        } else {
            "Very High".to_string()
        }
    }
    
    pub fn estimate_transaction_cost(
        &self,
        chain: &Chain,
        gas_units: u64,
        priority: &str,
    ) -> Option<Decimal> {
        let gas_price = self.get_current_gas_price(chain)?;
        
        let price_gwei = match priority {
            "slow" => gas_price.slow,
            "standard" => gas_price.standard,
            "fast" => gas_price.fast,
            _ => gas_price.standard,
        };
        
        let cost_eth = Decimal::from(gas_units) * price_gwei / Decimal::from(1_000_000_000);
        Some(cost_eth)
    }
    
    pub async fn start_monitoring(&self) {
        // This would start monitoring gas prices
        // Implementation would depend on actual chain integration
        tracing::info!("Gas monitoring started");
    }
}