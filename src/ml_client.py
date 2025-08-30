use std::sync::Arc;
use anyhow::Result;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::types::ArbitrageOpportunity;
use rust_decimal::prelude::ToPrimitive;
use chrono::Utc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MLRequest {
    #[serde(rename = "type")]
    request_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    opportunity: Option<OpportunityData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    opportunities: Option<Vec<OpportunityData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    targets: Option<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpportunityData {
    initial_amount: f64,
    roi_percentage: f64,
    path_length: f64,
    gas_cost: f64,
    flash_loan_fee: f64,
    hour: f64,
    day_of_week: f64,
    chain_id: f64,
    execution_time: f64,
    volume_ratio: f64,
    price_spread: f64,
    liquidity_depth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MLResponse {
    #[serde(rename = "type")]
    response_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scores: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    confidence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    loss: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    importance: Option<std::collections::HashMap<String, f64>>,
    timestamp: String,
}

pub struct MLClient {
    ws_url: String,
    connection: Arc<RwLock<Option<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>>,
}

impl MLClient {
    pub fn new() -> Self {
        Self {
            ws_url: "ws://127.0.0.1:8765".to_string(),
            connection: Arc::new(RwLock::new(None)),
        }
    }
    
    pub async fn connect(&self) -> Result<()> {
        let (ws_stream, _) = connect_async(&self.ws_url).await?;
        let mut conn = self.connection.write().await;
        *conn = Some(ws_stream);
        tracing::info!("Connected to Python ML engine at {}", self.ws_url);
        Ok(())
    }
    
    pub async fn predict(&self, opportunity: &ArbitrageOpportunity) -> Result<f64> {
        let opp_data = self.convert_opportunity(opportunity);
        
        let request = MLRequest {
            request_type: "predict".to_string(),
            opportunity: Some(opp_data),
            opportunities: None,
            targets: None,
        };
        
        let response = self.send_request(request).await?;
        
        Ok(response.score.unwrap_or(0.0))
    }
    
    pub async fn batch_predict(&self, opportunities: &[ArbitrageOpportunity]) -> Result<Vec<f64>> {
        let opp_data: Vec<OpportunityData> = opportunities
            .iter()
            .map(|o| self.convert_opportunity(o))
            .collect();
        
        let request = MLRequest {
            request_type: "batch_predict".to_string(),
            opportunity: None,
            opportunities: Some(opp_data),
            targets: None,
        };
        
        let response = self.send_request(request).await?;
        
        Ok(response.scores.unwrap_or_default())
    }
    
    pub async fn train(&self, opportunities: &[ArbitrageOpportunity]) -> Result<f64> {
        let opp_data: Vec<OpportunityData> = opportunities
            .iter()
            .map(|o| self.convert_opportunity(o))
            .collect();
        
        let targets: Vec<f64> = opportunities
            .iter()
            .map(|o| o.profit_usd)
            .collect();
        
        let request = MLRequest {
            request_type: "train".to_string(),
            opportunity: None,
            opportunities: Some(opp_data),
            targets: Some(targets),
        };
        
        let response = self.send_request(request).await?;
        
        Ok(response.loss.unwrap_or(0.0))
    }
    
    pub async fn get_feature_importance(&self) -> Result<std::collections::HashMap<String, f64>> {
        let request = MLRequest {
            request_type: "feature_importance".to_string(),
            opportunity: None,
            opportunities: None,
            targets: None,
        };
        
        let response = self.send_request(request).await?;
        
        Ok(response.importance.unwrap_or_default())
    }
    
    async fn send_request(&self, request: MLRequest) -> Result<MLResponse> {
        // Ensure we're connected
        if self.connection.read().await.is_none() {
            self.connect().await?;
        }
        
        let request_json = serde_json::to_string(&request)?;
        
        // Send request
        if let Some(ws) = &mut *self.connection.write().await {
            ws.send(Message::Text(request_json)).await?;
            
            // Wait for response
            if let Some(msg) = ws.next().await {
                match msg? {
                    Message::Text(text) => {
                        let response: MLResponse = serde_json::from_str(&text)?;
                        return Ok(response);
                    }
                    _ => {
                        return Err(anyhow::anyhow!("Unexpected message type"));
                    }
                }
            }
        }
        
        Err(anyhow::anyhow!("No connection available"))
    }
    
    fn convert_opportunity(&self, opp: &ArbitrageOpportunity) -> OpportunityData {
        use chrono::Timelike;
        use chrono::Datelike;
        
        OpportunityData {
            initial_amount: opp.initial_amount.to_f64().unwrap_or(0.0),
            roi_percentage: opp.roi_percentage,
            path_length: opp.path.len() as f64,
            gas_cost: opp.total_gas_cost.to_f64().unwrap_or(0.0),
            flash_loan_fee: opp.flash_loan_fee.to_f64().unwrap_or(0.0),
            hour: opp.timestamp.hour() as f64,
            day_of_week: opp.timestamp.weekday().num_days_from_monday() as f64,
            chain_id: match opp.chain {
                crate::types::Chain::Ethereum => 1.0,
                crate::types::Chain::BinanceSmartChain => 2.0,
                crate::types::Chain::Polygon => 3.0,
                crate::types::Chain::Arbitrum => 4.0,
                crate::types::Chain::Optimism => 5.0,
                crate::types::Chain::Avalanche => 6.0,
                crate::types::Chain::Fantom => 7.0,
                crate::types::Chain::Solana => 8.0,
                crate::types::Chain::Base => 9.0,
                _ => 0.0,
            },
            execution_time: opp.execution_time_ms as f64,
            volume_ratio: 1.0, // Would need actual volume data
            price_spread: opp.roi_percentage / 100.0,
            liquidity_depth: opp.initial_amount.to_f64().unwrap_or(0.0) * 100.0,
        }
    }
}

// Simplified ML Engine for Rust-only operation (fallback)
pub struct MetalMLEngine {
    client: Arc<MLClient>,
}

impl MetalMLEngine {
    pub fn new() -> Self {
        Self {
            client: Arc::new(MLClient::new()),
        }
    }
    
    pub async fn train(&self, data: &[ArbitrageOpportunity]) -> Result<()> {
        // Try to use Python ML, fall back to simple heuristics if unavailable
        match self.client.train(data).await {
            Ok(loss) => {
                tracing::info!("ML training completed with loss: {}", loss);
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Python ML unavailable: {}, using fallback", e);
                // Fallback: simple statistical model
                self.train_fallback(data).await
            }
        }
    }
    
    pub async fn predict(&self, opportunity: &ArbitrageOpportunity) -> f64 {
        // Try Python ML first, fall back to heuristics
        match self.client.predict(opportunity).await {
            Ok(score) => score,
            Err(_) => {
                // Simple heuristic-based scoring
                self.predict_fallback(opportunity)
            }
        }
    }
    
    async fn train_fallback(&self, _data: &[ArbitrageOpportunity]) -> Result<()> {
        // Simple statistical model training (placeholder)
        tracing::info!("Using fallback training method");
        Ok(())
    }
    
    fn predict_fallback(&self, opportunity: &ArbitrageOpportunity) -> f64 {
        // Simple heuristic scoring
        let mut score = 0.0;
        
        // Higher ROI is better
        score += opportunity.roi_percentage * 0.3;
        
        // Lower gas cost relative to profit is better
        let gas_ratio = opportunity.total_gas_cost.to_f64().unwrap_or(0.0) / 
                       opportunity.profit_usd.max(1.0);
        score += (1.0 - gas_ratio.min(1.0)) * 20.0;
        
        // Shorter paths are generally better
        score += (5.0 - opportunity.path.len() as f64).max(0.0) * 5.0;
        
        // Consider the chain (some chains have lower fees)
        score += match opportunity.chain {
            crate::types::Chain::Polygon | crate::types::Chain::Arbitrum => 10.0,
            crate::types::Chain::Ethereum => 5.0,
            _ => 7.0,
        };
        
        score.max(0.0).min(100.0)
    }
}