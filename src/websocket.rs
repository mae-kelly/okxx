use crate::types::*;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use chrono::Utc;
use tracing::{info, error};

pub struct PriceMonitor {
    state: Arc<SharedState>,
}

impl PriceMonitor {
    pub fn new(state: Arc<SharedState>) -> Self {
        Self { state }
    }
    
    pub async fn start(&self) -> Result<()> {
        // Start multiple price feeds
        let state1 = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_binance(state1).await {
                error!("Binance monitor error: {}", e);
            }
        });
        
        let state2 = self.state.clone();
        tokio::spawn(async move {
            if let Err(e) = Self::monitor_coinbase(state2).await {
                error!("Coinbase monitor error: {}", e);
            }
        });
        
        // Simulate DEX price feeds
        let state3 = self.state.clone();
        tokio::spawn(async move {
            Self::simulate_dex_prices(state3).await;
        });
        
        Ok(())
    }
    
    async fn monitor_binance(state: Arc<SharedState>) -> Result<()> {
        let url = "wss://stream.binance.com:9443/ws/!ticker@arr";
        let (ws_stream, _) = connect_async(url).await?;
        let (mut _write, mut read) = ws_stream.split();
        
        info!("Connected to Binance WebSocket");
        
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        if let Some(arr) = data.as_array() {
                            for item in arr {
                                if let (Some(symbol), Some(price)) = (
                                    item["s"].as_str(),
                                    item["c"].as_str(),
                                ) {
                                    if let Ok(price_decimal) = Decimal::from_str_exact(price) {
                                        let price_data = PriceData {
                                            token_pair: symbol.to_string(),
                                            price: price_decimal,
                                            liquidity: Decimal::from(1000000),
                                            volume_24h: Decimal::from(10000000),
                                            source: "Binance".to_string(),
                                            chain: Chain::BinanceSmartChain,
                                            timestamp: Utc::now(),
                                        };
                                        
                                        state.prices.insert(
                                            format!("binance_{}", symbol),
                                            price_data,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn monitor_coinbase(state: Arc<SharedState>) -> Result<()> {
        let url = "wss://ws-feed.exchange.coinbase.com";
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to ticker channel
        let subscribe_msg = json!({
            "type": "subscribe",
            "channels": ["ticker"],
            "product_ids": ["ETH-USD", "BTC-USD", "MATIC-USD"]
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await?;
        info!("Connected to Coinbase WebSocket");
        
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        if data["type"] == "ticker" {
                            if let (Some(product_id), Some(price)) = (
                                data["product_id"].as_str(),
                                data["price"].as_str(),
                            ) {
                                if let Ok(price_decimal) = Decimal::from_str_exact(price) {
                                    let price_data = PriceData {
                                        token_pair: product_id.to_string(),
                                        price: price_decimal,
                                        liquidity: Decimal::from(5000000),
                                        volume_24h: Decimal::from(50000000),
                                        source: "Coinbase".to_string(),
                                        chain: Chain::Ethereum,
                                        timestamp: Utc::now(),
                                    };
                                    
                                    state.prices.insert(
                                        format!("coinbase_{}", product_id),
                                        price_data,
                                    );
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn simulate_dex_prices(state: Arc<SharedState>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        
        loop {
            interval.tick().await;
            
            // Simulate Uniswap prices
            let eth_price = Decimal::from(2500) + Decimal::from(rand::random::<u32>() % 100) - Decimal::from(50);
            state.prices.insert(
                "uniswap_ETH_USDC".to_string(),
                PriceData {
                    token_pair: "ETH/USDC".to_string(),
                    price: eth_price,
                    liquidity: Decimal::from(50000000),
                    volume_24h: Decimal::from(100000000),
                    source: "Uniswap".to_string(),
                    chain: Chain::Ethereum,
                    timestamp: Utc::now(),
                },
            );
            
            // Simulate SushiSwap prices (slightly different)
            let sushi_eth_price = eth_price + Decimal::from(rand::random::<u32>() % 10) - Decimal::from(5);
            state.prices.insert(
                "sushiswap_ETH_USDC".to_string(),
                PriceData {
                    token_pair: "ETH/USDC".to_string(),
                    price: sushi_eth_price,
                    liquidity: Decimal::from(30000000),
                    volume_24h: Decimal::from(50000000),
                    source: "SushiSwap".to_string(),
                    chain: Chain::Ethereum,
                    timestamp: Utc::now(),
                },
            );
            
            // Simulate pools
            state.pools.insert(
                "uniswap_pool_1".to_string(),
                LiquidityPool {
                    address: "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640".to_string(),
                    token0: Token {
                        address: "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48".to_string(),
                        symbol: "USDC".to_string(),
                        decimals: 6,
                        chain: Chain::Ethereum,
                    },
                    token1: Token {
                        address: "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2".to_string(),
                        symbol: "WETH".to_string(),
                        decimals: 18,
                        chain: Chain::Ethereum,
                    },
                    reserve0: Decimal::from(100000000),
                    reserve1: Decimal::from(40000),
                    fee: Decimal::from_str_exact("0.003").unwrap(),
                    dex: "Uniswap V3".to_string(),
                    chain: Chain::Ethereum,
                    last_update: Utc::now(),
                },
            );
        }
    }
}

use rust_decimal::prelude::FromStr;