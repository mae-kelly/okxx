use crate::types::ArbitrageOpportunity;
use anyhow::Result;
use rocksdb::{DB, IteratorMode};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;

pub struct StorageEngine {
    db: Arc<DB>,
}

impl StorageEngine {
    pub fn new(path: &str) -> Result<Self> {
        let db = DB::open_default(path)?;
        Ok(Self { db: Arc::new(db) })
    }
    
    pub async fn store_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        let key = format!("opp_{}", opportunity.id);
        let value = bincode::serialize(opportunity)?;
        self.db.put(key.as_bytes(), &value)?;
        
        // Store index by chain
        let chain_key = format!("chain_{:?}_{}", opportunity.chain, opportunity.id);
        self.db.put(chain_key.as_bytes(), opportunity.id.as_bytes())?;
        
        // Store index by timestamp
        let time_key = format!("time_{}_{}", opportunity.timestamp.timestamp(), opportunity.id);
        self.db.put(time_key.as_bytes(), opportunity.id.as_bytes())?;
        
        Ok(())
    }
    
    pub async fn get_opportunity(&self, id: &str) -> Result<Option<ArbitrageOpportunity>> {
        let key = format!("opp_{}", id);
        if let Ok(Some(value)) = self.db.get(key.as_bytes()) {
            let opportunity = bincode::deserialize(&value)?;
            Ok(Some(opportunity))
        } else {
            Ok(None)
        }
    }
    
    pub async fn get_recent_opportunities(&self, hours: i64) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        let cutoff = Utc::now() - Duration::hours(hours);
        
        let iter = self.db.iterator(IteratorMode::Start);
        
        for item in iter {
            if let Ok((key, value)) = item {
                let key_str = String::from_utf8_lossy(&key);
                if key_str.starts_with("opp_") {
                    if let Ok(opp) = bincode::deserialize::<ArbitrageOpportunity>(&value) {
                        if opp.timestamp > cutoff {
                            opportunities.push(opp);
                        }
                    }
                }
            }
        }
        
        opportunities.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(opportunities)
    }
    
    pub async fn get_statistics(&self) -> Result<StorageStats> {
        let mut total_opportunities = 0;
        let mut profitable_opportunities = 0;
        let mut total_profit = Decimal::ZERO;
        let mut by_chain = std::collections::HashMap::new();
        
        let iter = self.db.iterator(IteratorMode::Start);
        
        for item in iter {
            if let Ok((key, value)) = item {
                let key_str = String::from_utf8_lossy(&key);
                if key_str.starts_with("opp_") {
                    if let Ok(opp) = bincode::deserialize::<ArbitrageOpportunity>(&value) {
                        total_opportunities += 1;
                        
                        if opp.net_profit_usd > Decimal::ZERO {
                            profitable_opportunities += 1;
                            total_profit += opp.net_profit_usd;
                        }
                        
                        *by_chain.entry(opp.chain).or_insert(0) += 1;
                    }
                }
            }
        }
        
        Ok(StorageStats {
            total_opportunities,
            profitable_opportunities,
            total_profit,
            opportunities_by_chain: by_chain,
        })
    }
    
    pub async fn cleanup_old_data(&self, days: i64) -> Result<()> {
        let cutoff = Utc::now() - Duration::days(days);
        let cutoff_timestamp = cutoff.timestamp();
        
        let iter = self.db.iterator(IteratorMode::Start);
        let mut keys_to_delete = Vec::new();
        
        for item in iter {
            if let Ok((key, _)) = item {
                let key_str = String::from_utf8_lossy(&key);
                if key_str.starts_with("time_") {
                    let parts: Vec<&str> = key_str.split('_').collect();
                    if parts.len() >= 2 {
                        if let Ok(timestamp) = parts[1].parse::<i64>() {
                            if timestamp < cutoff_timestamp {
                                keys_to_delete.push(key.to_vec());
                            }
                        }
                    }
                }
            }
        }
        
        for key in keys_to_delete {
            self.db.delete(&key)?;
        }
        
        Ok(())
    }
}

pub struct StorageStats {
    pub total_opportunities: usize,
    pub profitable_opportunities: usize,
    pub total_profit: rust_decimal::Decimal,
    pub opportunities_by_chain: std::collections::HashMap<crate::types::Chain, usize>,
}

use rust_decimal::Decimal;