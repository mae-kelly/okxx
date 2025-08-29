use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use rocksdb::{DB, IteratorMode};
use std::sync::Arc;
use crate::types::{ArbitrageOpportunity, MLInsights};

pub struct StorageEngine {
    db: Arc<DB>,
}

#[allow(dead_code)]impl StorageEngine {
    pub fn new(path: &str) -> Result<Self> {
        let db = DB::open_default(path)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub async fn store_opportunity(&self, opp: &ArbitrageOpportunity) -> Result<()> {
        let key = format!("opp_{}", opp.id);
        let value = bincode::serialize(opp)?;
        self.db.put(key.as_bytes(), &value)?;
        
        // Store index for querying by chain
        let chain_key = format!("idx_chain_{:?}_{}", opp.chain, opp.id);
        self.db.put(chain_key.as_bytes(), opp.id.as_bytes())?;
        
        // Store index for querying by time
        let time_key = format!("idx_time_{}_{}", opp.timestamp.timestamp(), opp.id);
        self.db.put(time_key.as_bytes(), opp.id.as_bytes())?;
        
        Ok(())
    }

    pub async fn get_opportunity(&self, id: &str) -> Result<Option<ArbitrageOpportunity>> {
        let key = format!("opp_{}", id);
        if let Ok(Some(value)) = self.db.get(key.as_bytes()) {
            let opp = bincode::deserialize(&value)?;
            Ok(Some(opp))
        } else {
            Ok(None)
        }
    }

    pub async fn get_recent_opportunities(&self, limit: usize) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        let iter = self.db.iterator(IteratorMode::End);
        let cutoff = Utc::now() - Duration::hours(24);
        
        for item in iter {
            if opportunities.len() >= limit {
                break;
            }
            
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
        
        Ok(opportunities)
    }

    pub async fn get_opportunities_by_chain(&self, chain: &crate::types::Chain, limit: usize) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        let prefix = format!("idx_chain_{:?}_", chain);
        let iter = self.db.prefix_iterator(prefix.as_bytes());
        
        for item in iter {
            if opportunities.len() >= limit {
                break;
            }
            
            if let Ok((_, id_bytes)) = item {
                let id = String::from_utf8_lossy(&id_bytes);
                if let Ok(Some(opp)) = self.get_opportunity(&id).await {
                    opportunities.push(opp);
                }
            }
        }
        
        Ok(opportunities)
    }

    pub async fn store_ml_insights(&self, insights: &MLInsights) -> Result<()> {
        let key = "latest_ml_insights";
        let value = bincode::serialize(insights)?;
        self.db.put(key.as_bytes(), &value)?;
        Ok(())
    }

    pub async fn get_latest_ml_insights(&self) -> Result<Option<MLInsights>> {
        if let Ok(Some(value)) = self.db.get(b"latest_ml_insights") {
            let insights = bincode::deserialize(&value)?;
            Ok(Some(insights))
        } else {
            Ok(None)
        }
    }

    pub async fn get_historical_data(&self, start: DateTime<Utc>, end: DateTime<Utc>) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();
        let start_key = format!("idx_time_{}", start.timestamp());
        let end_key = format!("idx_time_{}", end.timestamp());
        
        let iter = self.db.iterator(IteratorMode::From(start_key.as_bytes(), rocksdb::Direction::Forward));
        
        for item in iter {
            if let Ok((key, id_bytes)) = item {
                let key_str = String::from_utf8_lossy(&key);
                
                if !key_str.starts_with("idx_time_") || key_str.as_ref() > end_key.as_str() {
                    break;
                }
                
                let id = String::from_utf8_lossy(&id_bytes);
                if let Ok(Some(opp)) = self.get_opportunity(&id).await {
                    opportunities.push(opp);
                }
            }
        }
        
        Ok(opportunities)
    }

    pub async fn cleanup_old_data(&self, days_to_keep: i64) -> Result<()> {
        let cutoff = Utc::now() - Duration::days(days_to_keep);
        let cutoff_key = format!("idx_time_{}", cutoff.timestamp());
        
        let iter = self.db.iterator(IteratorMode::Start);
        let mut keys_to_delete = Vec::new();
        
        for item in iter {
            if let Ok((key, _)) = item {
                let key_str = String::from_utf8_lossy(&key);
                if key_str.starts_with("idx_time_") && key_str.as_ref() < cutoff_key.as_str() {
                    keys_to_delete.push(key.to_vec());
                }
            }
        }
        
        for key in keys_to_delete {
            self.db.delete(&key)?;
        }
        
        Ok(())
    }

    pub async fn get_statistics(&self) -> Result<StorageStatistics> {
        let mut total_opportunities = 0;
        let mut total_profit = 0.0;
        let mut chain_counts = std::collections::HashMap::new();
        
        let iter = self.db.iterator(IteratorMode::Start);
        
        for item in iter {
            if let Ok((key, value)) = item {
                let key_str = String::from_utf8_lossy(&key);
                if key_str.starts_with("opp_") {
                    if let Ok(opp) = bincode::deserialize::<ArbitrageOpportunity>(&value) {
                        total_opportunities += 1;
                        total_profit += opp.profit_usd;
                        *chain_counts.entry(format!("{:?}", opp.chain)).or_insert(0) += 1;
                    }
                }
            }
        }
        
        Ok(StorageStatistics {
            total_opportunities,
            total_profit,
            chain_counts,
            last_updated: Utc::now().timestamp(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStatistics {
    pub total_opportunities: usize,
    pub total_profit: f64,
    pub chain_counts: std::collections::HashMap<String, usize>,
    pub last_updated: i64,
}