use anyhow::Result;
use rust_decimal::Decimal;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use rand::Rng;  // Added proper rand import

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceUpdate {
    pub pair: String,
    pub bid: Decimal,
    pub ask: Decimal,
    pub timestamp: i64,
}

pub struct WebSocketManager {
    price_sender: mpsc::Sender<PriceUpdate>,
}

impl WebSocketManager {
    pub fn new(price_sender: mpsc::Sender<PriceUpdate>) -> Self {
        Self { price_sender }
    }

    pub async fn connect_binance(&self) -> Result<()> {
        let url = "wss://stream.binance.com:9443/ws/btcusdt@depth20@100ms";
        let (ws_stream, _) = connect_async(url).await?;
        
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to streams
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIBE",
            "params": [
                "btcusdt@depth20@100ms",
                "ethusdt@depth20@100ms"
            ],
            "id": 1
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await?;
        
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                        // Process price data
                        self.process_binance_data(data).await?;
                    }
                }
                Err(e) => {
                    eprintln!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }

    async fn process_binance_data(&self, data: serde_json::Value) -> Result<()> {
        // Process the data and extract price information
        if let Some(bids) = data["bids"].as_array() {
            if let Some(asks) = data["asks"].as_array() {
                if !bids.is_empty() && !asks.is_empty() {
                    let bid = Decimal::from_str_exact(
                        bids[0][0].as_str().unwrap_or("0")
                    ).unwrap_or_default();
                    
                    let ask = Decimal::from_str_exact(
                        asks[0][0].as_str().unwrap_or("0")
                    ).unwrap_or_default();
                    
                    let update = PriceUpdate {
                        pair: "BTC/USDT".to_string(),
                        bid,
                        ask,
                        timestamp: chrono::Utc::now().timestamp_millis(),
                    };
                    
                    self.price_sender.send(update).await?;
                }
            }
        }
        
        Ok(())
    }

    // Fixed: Using rand properly
    pub fn generate_mock_price(&self) -> Decimal {
        let mut rng = rand::thread_rng();
        let base = Decimal::from(50000);
        let variation = Decimal::from(rng.gen_range(-100..100));
        base + variation
    }
}