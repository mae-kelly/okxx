use crate::{config::Config, types::*};
use anyhow::Result;
use mongodb::{Client, Database, Collection, options::ClientOptions};
use redis::{Client as RedisClient, AsyncCommands};
use rust_decimal::Decimal;
use chrono::{Utc, Duration};
use serde_json;

pub struct DataStore {
    mongo_db: Database,
    redis_client: RedisClient,
    config: Config,
}

impl DataStore {
    pub async fn new(config: &Config) -> Result<Self> {
        let mongo_options = ClientOptions::parse(&config.database.mongodb_uri).await?;
        let mongo_client = Client::with_options(mongo_options)?;
        let mongo_db = mongo_client.database(&config.database.database_name);

        let redis_client = RedisClient::open(config.database.redis_uri.as_str())?;

        Ok(Self {
            mongo_db,
            redis_client,
            config: config.clone(),
        })
    }

    pub async fn store_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        collection.insert_one(opportunity, None).await?;

        let mut conn = self.redis_client.get_async_connection().await?;
        let key = format!("opportunity:{}", opportunity.id);
        let value = serde_json::to_string(opportunity)?;
        conn.setex(key, value, 3600).await?;

        Ok(())
    }

    pub async fn get_opportunity(&self, id: &str) -> Result<Option<ArbitrageOpportunity>> {
        let mut conn = self.redis_client.get_async_connection().await?;
        let key = format!("opportunity:{}", id);
        
        if let Ok(value) = conn.get::<_, String>(key).await {
            let opportunity: ArbitrageOpportunity = serde_json::from_str(&value)?;
            return Ok(Some(opportunity));
        }

        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let filter = mongodb::bson::doc! { "id": id };
        let result = collection.find_one(filter, None).await?;
        
        Ok(result)
    }

    pub async fn get_recent_opportunities(&self, limit: i64) -> Result<Vec<ArbitrageOpportunity>> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let options = mongodb::options::FindOptions::builder()
            .sort(mongodb::bson::doc! { "timestamp": -1 })
            .limit(limit)
            .build();
        
        let mut cursor = collection.find(None, options).await?;
        let mut opportunities = Vec::new();
        
        while cursor.advance().await? {
            opportunities.push(cursor.deserialize_current()?);
        }
        
        Ok(opportunities)
    }

    pub async fn count_opportunities_24h(&self) -> Result<u64> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let cutoff = Utc::now() - Duration::hours(24);
        let filter = mongodb::bson::doc! {
            "timestamp": { "$gte": cutoff.to_rfc3339() }
        };
        
        let count = collection.count_documents(filter, None).await?;
        Ok(count)
    }

    pub async fn count_profitable_opportunities_24h(&self) -> Result<u64> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let cutoff = Utc::now() - Duration::hours(24);
        let filter = mongodb::bson::doc! {
            "timestamp": { "$gte": cutoff.to_rfc3339() },
            "net_profit": { "$gt": 0 }
        };
        
        let count = collection.count_documents(filter, None).await?;
        Ok(count)
    }

    pub async fn get_total_profit_24h(&self) -> Result<Decimal> {
        let opportunities = self.get_opportunities_since(Utc::now() - Duration::hours(24)).await?;
        
        let total: Decimal = opportunities
            .iter()
            .map(|o| o.net_profit)
            .filter(|p| p > &Decimal::ZERO)
            .sum();
        
        Ok(total)
    }

    async fn get_opportunities_since(&self, since: chrono::DateTime<Utc>) -> Result<Vec<ArbitrageOpportunity>> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let filter = mongodb::bson::doc! {
            "timestamp": { "$gte": since.to_rfc3339() }
        };
        
        let mut cursor = collection.find(filter, None).await?;
        let mut opportunities = Vec::new();
        
        while cursor.advance().await? {
            opportunities.push(cursor.deserialize_current()?);
        }
        
        Ok(opportunities)
    }

    pub async fn get_training_data(&self, limit: usize) -> Result<Vec<ArbitrageOpportunity>> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let options = mongodb::options::FindOptions::builder()
            .sort(mongodb::bson::doc! { "timestamp": -1 })
            .limit(limit as i64)
            .build();
        
        let mut cursor = collection.find(None, options).await?;
        let mut data = Vec::new();
        
        while cursor.advance().await? {
            data.push(cursor.deserialize_current()?);
        }
        
        Ok(data)
    }

    pub async fn store_price(&self, price: &Price) -> Result<()> {
        let mut conn = self.redis_client.get_async_connection().await?;
        
        let key = format!("price:{}:{}:{}", 
            price.exchange, 
            price.pair.base.symbol, 
            price.pair.quote.symbol
        );
        
        let value = serde_json::to_string(price)?;
        conn.setex(key, value, 60).await?;
        
        Ok(())
    }

    pub async fn get_price(&self, exchange: &str, pair: &TokenPair) -> Result<Option<Price>> {
        let mut conn = self.redis_client.get_async_connection().await?;
        
        let key = format!("price:{}:{}:{}", 
            exchange, 
            pair.base.symbol, 
            pair.quote.symbol
        );
        
        if let Ok(value) = conn.get::<_, String>(key).await {
            let price: Price = serde_json::from_str(&value)?;
            return Ok(Some(price));
        }
        
        Ok(None)
    }

    pub async fn cleanup_old_data(&self) -> Result<()> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let cutoff = Utc::now() - Duration::days(self.config.database.retention_days as i64);
        let filter = mongodb::bson::doc! {
            "timestamp": { "$lt": cutoff.to_rfc3339() }
        };
        
        collection.delete_many(filter, None).await?;
        
        Ok(())
    }

    pub async fn get_exchange_statistics(&self, exchange: &str) -> Result<ExchangeStatistics> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let filter = mongodb::bson::doc! {
            "$or": [
                { "buy_exchange": exchange },
                { "sell_exchange": exchange }
            ]
        };
        
        let total_opportunities = collection.count_documents(filter.clone(), None).await?;
        
        let profitable_filter = mongodb::bson::doc! {
            "$and": [
                filter,
                { "net_profit": { "$gt": 0 } }
            ]
        };
        
        let profitable_opportunities = collection.count_documents(profitable_filter, None).await?;
        
        Ok(ExchangeStatistics {
            exchange: exchange.to_string(),
            total_opportunities,
            profitable_opportunities,
            success_rate: if total_opportunities > 0 {
                profitable_opportunities as f64 / total_opportunities as f64
            } else {
                0.0
            },
        })
    }

    pub async fn get_pair_statistics(&self, pair: &TokenPair) -> Result<PairStatistics> {
        let collection: Collection<ArbitrageOpportunity> = 
            self.mongo_db.collection("opportunities");
        
        let filter = mongodb::bson::doc! {
            "token_pair.base.symbol": &pair.base.symbol,
            "token_pair.quote.symbol": &pair.quote.symbol,
        };
        
        let mut cursor = collection.find(filter, None).await?;
        let mut opportunities = Vec::new();
        
        while cursor.advance().await? {
            opportunities.push(cursor.deserialize_current()?);
        }
        
        let total_opportunities = opportunities.len() as u64;
        let profitable_opportunities = opportunities
            .iter()
            .filter(|o| o.net_profit > Decimal::ZERO)
            .count() as u64;
        
        let avg_profit = if !opportunities.is_empty() {
            opportunities
                .iter()
                .map(|o| o.net_profit)
                .sum::<Decimal>() / Decimal::from(opportunities.len())
        } else {
            Decimal::ZERO
        };
        
        let max_profit = opportunities
            .iter()
            .map(|o| o.net_profit)
            .max()
            .unwrap_or(Decimal::ZERO);
        
        Ok(PairStatistics {
            pair: pair.clone(),
            total_opportunities,
            profitable_opportunities,
            avg_profit,
            max_profit,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ExchangeStatistics {
    pub exchange: String,
    pub total_opportunities: u64,
    pub profitable_opportunities: u64,
    pub success_rate: f64,
}

#[derive(Debug, Clone)]
pub struct PairStatistics {
    pub pair: TokenPair,
    pub total_opportunities: u64,
    pub profitable_opportunities: u64,
    pub avg_profit: Decimal,
    pub max_profit: Decimal,
}