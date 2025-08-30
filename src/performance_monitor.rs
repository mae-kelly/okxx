use std::sync::Arc;
use chrono::{DateTime, Utc, Timelike};
use rust_decimal::Decimal;
use colored::*;
use crate::types::{SharedState, MarketSignal};
use rust_decimal::prelude::ToPrimitive;

pub struct PerformanceMonitor {
    state: Arc<SharedState>,
    start_time: DateTime<Utc>,
}

pub struct PerformanceStats {
    pub total_profit: f64,
    pub success_rate: f64,
    pub active_opportunities: usize,
}

impl PerformanceMonitor {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self {
            state,
            start_time: Utc::now(),
        }
    }
    
    pub async fn update_metrics(&self) {
        // Update metrics based on current state
        let opportunities = self.state.opportunities.read().await;
        let total_opportunities = opportunities.len();
        let profitable = opportunities.iter()
            .filter(|o| o.profit_usd > 0.0)
            .count();
        
        if total_opportunities > 0 {
            let success_rate = profitable as f64 / total_opportunities as f64;
            tracing::debug!(
                "Metrics updated: {} opportunities, {:.2}% profitable",
                total_opportunities,
                success_rate * 100.0
            );
        }
    }
    
    pub async fn get_statistics(&self) -> PerformanceStats {
        let opportunities = self.state.opportunities.read().await;
        
        let total_profit = opportunities.iter()
            .map(|o| o.profit_usd)
            .sum::<f64>();
        
        let success_rate = if !opportunities.is_empty() {
            opportunities.iter()
                .filter(|o| o.profit_usd > 0.0)
                .count() as f64 / opportunities.len() as f64
        } else {
            0.0
        };
        
        PerformanceStats {
            total_profit,
            success_rate,
            active_opportunities: opportunities.len(),
        }
    }
    
    pub async fn print_dashboard(&self) {
        println!("\n{}", "═".repeat(80).bright_blue());
        println!("{}", "📊 ARBITRAGE SCANNER PERFORMANCE DASHBOARD".bright_white().bold());
        println!("{}", "═".repeat(80).bright_blue());
        
        // Get signals from performance_stats
        let signals = self.state.performance_stats.len() as u64;
        let uptime = (Utc::now() - self.start_time).num_seconds();
        
        let total_profit: Decimal = self.state.performance_stats
            .iter()
            .map(|entry| entry.value().profit)
            .sum();
        
        if let Some(best_entry) = self.state.performance_stats
            .iter()
            .max_by_key(|entry| (entry.value().profit.to_f64().unwrap_or(0.0) * 1000.0) as i64) 
        {
            let best = best_entry.value();
            println!(
                "\n{} Best Opportunity: {} → {} | Profit: ${} | ROI: {:.2}%",
                "🎯".bright_yellow(),
                best.buy_exchange.bright_cyan(),
                best.sell_exchange.bright_cyan(),
                best.profit.to_string().bright_green().bold(),
                best.roi
            );
        }
        
        println!("\n{} Statistics:", "📈".bright_yellow());
        println!("  • Signals Processed: {}", signals.to_string().bright_white());
        println!("  • Total Profit: ${}", total_profit.to_string().bright_green().bold());
        println!("  • Uptime: {} seconds", uptime.to_string().bright_white());
        
        println!("\n{}", "─".repeat(80).bright_blue());
    }
    
    pub async fn get_hourly_stats(&self) -> Vec<(u8, Decimal)> {
        let mut hourly_profits: std::collections::HashMap<u8, Decimal> = std::collections::HashMap::new();
        
        for entry in self.state.performance_stats.iter() {
            let hour = entry.value().timestamp.hour() as u8;
            *hourly_profits.entry(hour).or_insert(Decimal::ZERO) += entry.value().profit;
        }
        
        let mut results: Vec<(u8, Decimal)> = hourly_profits.into_iter().collect();
        results.sort_by_key(|&(hour, _)| hour);
        results
    }
    
    pub async fn get_exchange_performance(&self) -> Vec<(String, Decimal)> {
        let mut exchange_profits: std::collections::HashMap<String, Decimal> = std::collections::HashMap::new();
        
        for entry in self.state.performance_stats.iter() {
            let signal = entry.value();
            *exchange_profits.entry(signal.buy_exchange.clone()).or_insert(Decimal::ZERO) += signal.profit / Decimal::from(2);
            *exchange_profits.entry(signal.sell_exchange.clone()).or_insert(Decimal::ZERO) += signal.profit / Decimal::from(2);
        }
        
        let mut results: Vec<(String, Decimal)> = exchange_profits.into_iter().collect();
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }
    
    pub async fn print_top_opportunities(&self, limit: usize) {
        println!("\n{} Top {} Opportunities:", "🏆".bright_yellow(), limit);
        
        let mut signals: Vec<MarketSignal> = self.state.performance_stats
            .iter()
            .map(|entry| entry.value().clone())
            .collect();
        
        signals.sort_by(|a, b| b.profit.partial_cmp(&a.profit).unwrap());
        signals.truncate(limit);
        
        for (i, signal) in signals.iter().enumerate() {
            println!(
                "  {}. {} → {} | Profit: ${} | ROI: {:.2}%",
                i + 1,
                signal.buy_exchange.bright_cyan(),
                signal.sell_exchange.bright_cyan(),
                signal.profit.to_string().bright_green(),
                signal.roi
            );
        }
    }
}