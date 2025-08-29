use std::sync::Arc;
use anyhow::Result;
use smartcore::ensemble::random_forest_regressor::RandomForestRegressor;
use smartcore::linalg::basic::matrix::DenseMatrix;
use ndarray::{Array2, s};
use chrono::{Utc, Datelike, Timelike};
use std::collections::HashMap;
use rust_decimal::prelude::ToPrimitive;
use crate::types::{ArbitrageOpportunity, MLInsights, TimeWindow, Chain};
use crate::storage::StorageEngine;

pub struct MLAnalyzer {
    storage: Arc<StorageEngine>,
    models: HashMap<String, RandomForestRegressor<f64, f64, DenseMatrix<f64>, Vec<f64>>>,
}

#[allow(dead_code)]impl MLAnalyzer {
    pub fn new(storage: Arc<StorageEngine>) -> Result<Self> {
        Ok(Self {
            storage,
            models: HashMap::new(),
        })
    }
    
    pub async fn analyze_patterns(&self, opportunities: &[ArbitrageOpportunity]) -> Result<MLInsights> {
        let chain_profits = self.analyze_chain_profitability(opportunities);
        let exchange_profits = self.analyze_exchange_profitability(opportunities);
        let token_profits = self.analyze_token_profitability(opportunities);
        let time_windows = self.analyze_time_patterns(opportunities);
        let frequency = self.analyze_opportunity_frequency(opportunities);
        
        let features = self.extract_features(opportunities);
        let predictions = self.predict_future_opportunities(&features).await?;
        
        Ok(MLInsights {
            most_profitable_chains: chain_profits,
            most_profitable_exchanges: exchange_profits,
            most_profitable_tokens: token_profits,
            best_time_windows: time_windows,
            average_profit_by_chain: self.calculate_average_profits_by_chain(opportunities),
            opportunity_frequency: frequency,
            prediction_accuracy: predictions,
            generated_at: Utc::now(),
        })
    }
    
    fn analyze_chain_profitability(&self, opportunities: &[ArbitrageOpportunity]) -> Vec<(Chain, f64)> {
        let mut chain_profits: HashMap<Chain, Vec<f64>> = HashMap::new();
        
        for opp in opportunities {
            chain_profits.entry(opp.chain.clone())
                .or_insert_with(Vec::new)
                .push(opp.profit_usd);
        }
        
        let mut results: Vec<(Chain, f64)> = chain_profits.into_iter()
            .map(|(chain, profits)| {
                let total: f64 = profits.iter().sum();
                (chain, total)
            })
            .collect();
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(10);
        results
    }
    
    fn analyze_exchange_profitability(&self, opportunities: &[ArbitrageOpportunity]) -> Vec<(String, f64)> {
        let mut exchange_profits: HashMap<String, Vec<f64>> = HashMap::new();
        
        for opp in opportunities {
            for leg in &opp.path {
                exchange_profits.entry(leg.exchange.clone())
                    .or_insert_with(Vec::new)
                    .push(opp.profit_usd / opp.path.len() as f64);
            }
        }
        
        let mut results: Vec<(String, f64)> = exchange_profits.into_iter()
            .map(|(exchange, profits)| {
                let total: f64 = profits.iter().sum();
                (exchange, total)
            })
            .collect();
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(20);
        results
    }
    
    fn analyze_token_profitability(&self, opportunities: &[ArbitrageOpportunity]) -> Vec<(String, f64)> {
        let mut token_profits: HashMap<String, Vec<f64>> = HashMap::new();
        
        for opp in opportunities {
            for leg in &opp.path {
                token_profits.entry(leg.token_in.clone())
                    .or_insert_with(Vec::new)
                    .push(opp.profit_usd / (opp.path.len() * 2) as f64);
                
                token_profits.entry(leg.token_out.clone())
                    .or_insert_with(Vec::new)
                    .push(opp.profit_usd / (opp.path.len() * 2) as f64);
            }
        }
        
        let mut results: Vec<(String, f64)> = token_profits.into_iter()
            .filter(|(_, profits)| profits.len() > 5)
            .map(|(token, profits)| {
                let total: f64 = profits.iter().sum();
                (token, total)
            })
            .collect();
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results.truncate(30);
        results
    }
    
    fn analyze_time_patterns(&self, opportunities: &[ArbitrageOpportunity]) -> Vec<TimeWindow> {
        let mut time_buckets: HashMap<(u8, u8), Vec<f64>> = HashMap::new();
        
        for opp in opportunities {
            let hour = opp.timestamp.hour() as u8;
            let day = opp.timestamp.weekday().num_days_from_monday() as u8;
            
            time_buckets.entry((hour, day))
                .or_insert_with(Vec::new)
                .push(opp.profit_usd);
        }
        
        let mut windows: Vec<TimeWindow> = time_buckets.into_iter()
            .map(|((hour, day), profits)| {
                let count = profits.len() as f64;
                let avg_profit: f64 = profits.iter().sum::<f64>() / count;
                
                TimeWindow {
                    hour,
                    day_of_week: day,
                    avg_opportunities: count,
                    avg_profit,
                }
            })
            .collect();
        
        windows.sort_by(|a, b| b.avg_profit.partial_cmp(&a.avg_profit).unwrap());
        windows.truncate(24);
        windows
    }
    
    fn analyze_opportunity_frequency(&self, opportunities: &[ArbitrageOpportunity]) -> Vec<(String, u64)> {
        let mut frequency: HashMap<String, u64> = HashMap::new();
        
        for opp in opportunities {
            let key = format!("{:?}_{}", opp.chain, opp.path.first().map(|l| &l.exchange).unwrap_or(&String::new()));
            *frequency.entry(key).or_insert(0) += 1;
        }
        
        let mut results: Vec<(String, u64)> = frequency.into_iter().collect();
        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.truncate(20);
        results
    }
    
    fn calculate_average_profits_by_chain(&self, opportunities: &[ArbitrageOpportunity]) -> Vec<(Chain, f64)> {
        let mut chain_stats: HashMap<Chain, (f64, usize)> = HashMap::new();
        
        for opp in opportunities {
            let entry = chain_stats.entry(opp.chain.clone()).or_insert((0.0, 0));
            entry.0 += opp.profit_usd;
            entry.1 += 1;
        }
        
        let mut results: Vec<(Chain, f64)> = chain_stats.into_iter()
            .map(|(chain, (total, count))| {
                (chain, total / count as f64)
            })
            .collect();
        
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        results
    }
    
    fn extract_features(&self, opportunities: &[ArbitrageOpportunity]) -> Array2<f64> {
        let n_samples = opportunities.len();
        let n_features = 10;
        let mut features = Array2::zeros((n_samples, n_features));
        
        for (i, opp) in opportunities.iter().enumerate() {
            features[[i, 0]] = opp.initial_amount.to_f64().unwrap_or(0.0);
            features[[i, 1]] = opp.roi_percentage;
            features[[i, 2]] = opp.path.len() as f64;
            features[[i, 3]] = opp.total_gas_cost.to_f64().unwrap_or(0.0);
            features[[i, 4]] = opp.flash_loan_fee.to_f64().unwrap_or(0.0);
            features[[i, 5]] = opp.timestamp.hour() as f64;
            features[[i, 6]] = opp.timestamp.weekday().num_days_from_monday() as f64;
            features[[i, 7]] = match opp.chain {
                Chain::Ethereum => 1.0,
                Chain::BinanceSmartChain => 2.0,
                Chain::Polygon => 3.0,
                Chain::Arbitrum => 4.0,
                Chain::Optimism => 5.0,
                Chain::Avalanche => 6.0,
                Chain::Fantom => 7.0,
                Chain::Solana => 8.0,
                Chain::Base => 9.0,
                Chain::ZkSync => 10.0,
                Chain::Linea => 11.0,
                Chain::Scroll => 12.0,
                Chain::Blast => 13.0,
            };
            features[[i, 8]] = opp.execution_time_ms as f64;
            features[[i, 9]] = opp.profit_usd;
        }
        
        features
    }
    
    async fn predict_future_opportunities(&self, features: &Array2<f64>) -> Result<f64> {
        if features.nrows() < 10 {
            return Ok(0.0);
        }
        
        let n_samples = features.nrows();
        let split_index = (n_samples as f64 * 0.8) as usize;
        
        let x_train = DenseMatrix::from_2d_array(
            &features.slice(s![..split_index, ..9])
                .outer_iter()
                .map(|row| row.to_vec())
                .collect::<Vec<_>>()
                .iter()
                .map(|v| v.as_slice())
                .collect::<Vec<_>>()
        ).unwrap();
        let y_train: Vec<f64> = features.slice(s![..split_index, 9]).to_owned().into_raw_vec_and_offset().0;
        
        let x_test = DenseMatrix::from_2d_array(
            &features.slice(s![split_index.., ..9])
                .outer_iter()
                .map(|row| row.to_vec())
                .collect::<Vec<_>>()
                .iter()
                .map(|v| v.as_slice())
                .collect::<Vec<_>>()
        ).unwrap();
        let y_test: Vec<f64> = features.slice(s![split_index.., 9]).to_owned().into_raw_vec_and_offset().0;
        
        let model = RandomForestRegressor::fit(
            &x_train,
            &y_train,
            Default::default()
        )?;
        
        let predictions = model.predict(&x_test)?;
        
        let mut squared_errors = 0.0;
        let mut total_actual = 0.0;
        
        for (pred, actual) in predictions.iter().zip(y_test.iter()) {
            squared_errors += (pred - actual).powi(2);
            total_actual += actual.abs();
        }
        
        let mse: f64 = squared_errors / y_test.len() as f64;
        let rmse = mse.sqrt();
        let avg_actual = total_actual / y_test.len() as f64;
        
        let accuracy = if avg_actual > 0.0 {
            1.0 - (rmse / avg_actual).min(1.0)
        } else {
            0.0
        };
        
        Ok(accuracy * 100.0)
    }
    
    pub async fn train_models(&mut self, historical_data: &[ArbitrageOpportunity]) -> Result<()> {
        let features = self.extract_features(historical_data);
        
        if features.nrows() < 100 {
            return Ok(());
        }
        
        let chains = vec![
            Chain::Ethereum,
            Chain::BinanceSmartChain,
            Chain::Polygon,
            Chain::Arbitrum,
            Chain::Optimism,
        ];
        
        for chain in chains {
            let chain_data: Vec<usize> = historical_data.iter()
                .enumerate()
                .filter(|(_, opp)| opp.chain == chain)
                .map(|(i, _)| i)
                .collect();
            
            if chain_data.len() > 20 {
                let mut chain_features = Array2::zeros((chain_data.len(), 9));
                let mut chain_targets = Vec::new();
                
                for (new_i, &orig_i) in chain_data.iter().enumerate() {
                    for j in 0..9 {
                        chain_features[[new_i, j]] = features[[orig_i, j]];
                    }
                    chain_targets.push(features[[orig_i, 9]]);
                }
                
                let x = DenseMatrix::from_2d_array(
                    &chain_features
                        .outer_iter()
                        .map(|row| row.to_vec())
                        .collect::<Vec<_>>()
                        .iter()
                        .map(|v| v.as_slice())
                        .collect::<Vec<_>>()
                ).unwrap();
                
                let model = RandomForestRegressor::fit(
                    &x,
                    &chain_targets,
                    Default::default()
                )?;
                
                self.models.insert(format!("{:?}", chain), model);
            }
        }
        
        Ok(())
    }
}