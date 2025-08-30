use crate::types::*;
use super::Exchange;
use anyhow::Result;
use async_trait::async_trait;
use rust_decimal::Decimal;
use chrono::{Utc, DateTime};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use base64::{Engine as _, engine::general_purpose};
use reqwest::{Client, header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use tracing::{info, debug, warn, error};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct OkxConfig {
    pub api_key: String,
    pub secret_key: String,
    pub passphrase: String,
    pub ws_public_url: String,
    pub ws_private_url: String,
    pub rest_url: String,
}

impl OkxConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            api_key: std::env::var("OKX_API_KEY")?,
            secret_key: std::env::var("OKX_SECRET_KEY")?,
            passphrase: std::env::var("OKX_PASSPHRASE").unwrap_or_default(),
            ws_public_url: std::env::var("OKX_WS_PUBLIC")
                .unwrap_or_else(|_| "wss://ws.okx.com:8443/ws/v5/public".to_string()),
            ws_private_url: std::env::var("OKX_WS_PRIVATE")
                .unwrap_or_else(|_| "wss://ws.okx.com:8443/ws/v5/private".to_string()),
            rest_url: std::env::var("OKX_REST_URL")
                .unwrap_or_else(|_| "https://www.okx.com".to_string()),
        })
    }
}

pub struct OkxExchange {
    config: OkxConfig,
    client: Client,
    pair_cache: Arc<RwLock<HashMap<String, TokenPair>>>,
    price_cache: Arc<RwLock<HashMap<String, Price>>>,
}

#[derive(Debug, Deserialize)]
struct OkxResponse<T> {
    code: String,
    msg: String,
    data: Vec<T>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OkxTicker {
    #[serde(rename = "instId")]
    inst_id: String,
    #[serde(rename = "last")]
    last_price: String,
    #[serde(rename = "askPx")]
    ask_price: String,
    #[serde(rename = "bidPx")]
    bid_price: String,
    #[serde(rename = "askSz")]
    ask_size: String,
    #[serde(rename = "bidSz")]
    bid_size: String,
    #[serde(rename = "vol24h")]
    volume_24h: String,
    #[serde(rename = "volCcy24h")]
    volume_ccy_24h: String,
}

#[derive(Debug, Deserialize)]
struct OkxInstrument {
    #[serde(rename = "instId")]
    inst_id: String,
    #[serde(rename = "baseCcy")]
    base_ccy: String,
    #[serde(rename = "quoteCcy")]
    quote_ccy: String,
    #[serde(rename = "state")]
    state: String,
}

impl OkxExchange {
    pub async fn new() -> Result<Self> {
        let config = OkxConfig::from_env()?;
        let client = Client::new();
        
        info!("Initializing OKX exchange with API key: {}...", &config.api_key[..8]);
        
        Ok(Self {
            config,
            client,
            pair_cache: Arc::new(RwLock::new(HashMap::new())),
            price_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    fn sign_request(&self, timestamp: &str, method: &str, path: &str, body: &str) -> String {
        let message = format!("{}{}{}{}", timestamp, method, path, body);
        let mut mac = HmacSha256::new_from_slice(self.config.secret_key.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(message.as_bytes());
        let result = mac.finalize();
        general_purpose::STANDARD.encode(result.into_bytes())
    }
    
    async fn make_request<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<OkxResponse<T>> {
        let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%S.%3fZ").to_string();
        let method = "GET";
        let full_path = format!("/api/v5{}", path);
        let signature = self.sign_request(&timestamp, method, &full_path, "");
        
        let url = format!("{}{}", self.config.rest_url, full_path);
        
        let response = self.client
            .get(&url)
            .header("OK-ACCESS-KEY", &self.config.api_key)
            .header("OK-ACCESS-SIGN", signature)
            .header("OK-ACCESS-TIMESTAMP", timestamp)
            .header("OK-ACCESS-PASSPHRASE", &self.config.passphrase)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        
        let text = response.text().await?;
        let result: OkxResponse<T> = serde_json::from_str(&text)?;
        
        if result.code != "0" {
            return Err(anyhow::anyhow!("OKX API error: {} - {}", result.code, result.msg));
        }
        
        Ok(result)
    }
    
    pub async fn start_websocket(&self) {
        let url = url::Url::parse(&self.config.ws_public_url).unwrap();
        let (ws_stream, _) = connect_async(url).await.expect("Failed to connect to OKX WebSocket");
        let (mut write, mut read) = ws_stream.split();
        
        // Subscribe to tickers
        let subscribe_msg = json!({
            "op": "subscribe",
            "args": [
                {
                    "channel": "tickers",
                    "instId": "BTC-USDT"
                },
                {
                    "channel": "tickers",
                    "instId": "ETH-USDT"
                },
                {
                    "channel": "books5",
                    "instId": "BTC-USDT"
                },
                {
                    "channel": "books5",
                    "instId": "ETH-USDT"
                }
            ]
        });
        
        write.send(Message::Text(subscribe_msg.to_string())).await.unwrap();
        info!("Subscribed to OKX WebSocket channels");
        
        let price_cache = self.price_cache.clone();
        
        tokio::spawn(async move {
            while let Some(message) = read.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            if let Some(event) = data.get("event") {
                                if event == "subscribe" {
                                    info!("Successfully subscribed to OKX channel");
                                }
                            } else if let Some(arg) = data.get("arg") {
                                if let Some(channel) = arg.get("channel") {
                                    if channel == "tickers" {
                                        Self::process_ticker_update(&data, &price_cache).await;
                                    }
                                }
                            }
                        }
                    }
                    Ok(Message::Ping(ping)) => {
                        debug!("Received ping from OKX, sending pong");
                    }
                    Err(e) => {
                        error!("OKX WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
    
    async fn process_ticker_update(data: &Value, price_cache: &Arc<RwLock<HashMap<String, Price>>>) {
        if let Some(data_array) = data.get("data").and_then(|d| d.as_array()) {
            for item in data_array {
                if let Ok(ticker) = serde_json::from_value::<OkxTicker>(item.clone()) {
                    let pair = Self::parse_pair(&ticker.inst_id);
                    
                    if let (Ok(bid), Ok(ask), Ok(bid_size), Ok(ask_size)) = (
                        Decimal::from_str_exact(&ticker.bid_price),
                        Decimal::from_str_exact(&ticker.ask_price),
                        Decimal::from_str_exact(&ticker.bid_size),
                        Decimal::from_str_exact(&ticker.ask_size),
                    ) {
                        let price = Price {
                            bid,
                            ask,
                            bid_size,
                            ask_size,
                            timestamp: Utc::now(),
                            exchange: "OKX".to_string(),
                            pair,
                        };
                        
                        let mut cache = price_cache.write().await;
                        cache.insert(ticker.inst_id.clone(), price);
                        debug!("Updated price for {}: bid={}, ask={}", ticker.inst_id, bid, ask);
                    }
                }
            }
        }
    }
    
    fn parse_pair(inst_id: &str) -> TokenPair {
        let parts: Vec<&str> = inst_id.split('-').collect();
        if parts.len() >= 2 {
            TokenPair {
                base: Token {
                    address: String::new(),
                    symbol: parts[0].to_string(),
                    decimals: 18,
                    chain_id: 1,
                },
                quote: Token {
                    address: String::new(),
                    symbol: parts[1].to_string(),
                    decimals: 18,
                    chain_id: 1,
                },
            }
        } else {
            TokenPair {
                base: Token {
                    address: String::new(),
                    symbol: inst_id.to_string(),
                    decimals: 18,
                    chain_id: 1,
                },
                quote: Token {
                    address: String::new(),
                    symbol: "USDT".to_string(),
                    decimals: 18,
                    chain_id: 1,
                },
            }
        }
    }
}

#[async_trait]
impl Exchange for OkxExchange {
    async fn get_name(&self) -> String {
        "OKX".to_string()
    }
    
    async fn get_pairs(&self) -> Result<Vec<TokenPair>> {
        let response: OkxResponse<OkxInstrument> = self.make_request("/public/instruments?instType=SPOT").await?;
        
        let mut pairs = Vec::new();
        for instrument in response.data {
            if instrument.state == "live" {
                let pair = TokenPair {
                    base: Token {
                        address: String::new(),
                        symbol: instrument.base_ccy,
                        decimals: 18,
                        chain_id: 1,
                    },
                    quote: Token {
                        address: String::new(),
                        symbol: instrument.quote_ccy,
                        decimals: 18,
                        chain_id: 1,
                    },
                };
                
                let mut cache = self.pair_cache.write().await;
                cache.insert(instrument.inst_id.clone(), pair.clone());
                pairs.push(pair);
            }
        }
        
        info!("Loaded {} trading pairs from OKX", pairs.len());
        Ok(pairs)
    }
    
    async fn get_price(&self, pair: &TokenPair) -> Result<Price> {
        let inst_id = format!("{}-{}", pair.base.symbol, pair.quote.symbol);
        
        // Check cache first
        {
            let cache = self.price_cache.read().await;
            if let Some(price) = cache.get(&inst_id) {
                if (Utc::now() - price.timestamp).num_seconds() < 5 {
                    return Ok(price.clone());
                }
            }
        }
        
        // Fetch from API
        let path = format!("/market/ticker?instId={}", inst_id);
        let response: OkxResponse<OkxTicker> = self.make_request(&path).await?;
        
        if response.data.is_empty() {
            return Err(anyhow::anyhow!("No ticker data for {}", inst_id));
        }
        
        let ticker = &response.data[0];
        let price = Price {
            bid: Decimal::from_str_exact(&ticker.bid_price)?,
            ask: Decimal::from_str_exact(&ticker.ask_price)?,
            bid_size: Decimal::from_str_exact(&ticker.bid_size)?,
            ask_size: Decimal::from_str_exact(&ticker.ask_size)?,
            timestamp: Utc::now(),
            exchange: "OKX".to_string(),
            pair: pair.clone(),
        };
        
        // Update cache
        let mut cache = self.price_cache.write().await;
        cache.insert(inst_id, price.clone());
        
        Ok(price)
    }
    
    async fn get_orderbook(&self, pair: &TokenPair, depth: usize) -> Result<OrderBook> {
        let inst_id = format!("{}-{}", pair.base.symbol, pair.quote.symbol);
        let path = format!("/market/books?instId={}&sz={}", inst_id, depth);
        
        let response = self.client
            .get(format!("{}/api/v5{}", self.config.rest_url, path))
            .send()
            .await?;
        
        let data: Value = response.json().await?;
        
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        
        if let Some(book_data) = data["data"][0]["bids"].as_array() {
            for bid in book_data {
                if let (Some(price), Some(qty)) = (bid[0].as_str(), bid[1].as_str()) {
                    bids.push(Order {
                        price: Decimal::from_str_exact(price)?,
                        quantity: Decimal::from_str_exact(qty)?,
                        timestamp: Utc::now(),
                    });
                }
            }
        }
        
        if let Some(book_data) = data["data"][0]["asks"].as_array() {
            for ask in book_data {
                if let (Some(price), Some(qty)) = (ask[0].as_str(), ask[1].as_str()) {
                    asks.push(Order {
                        price: Decimal::from_str_exact(price)?,
                        quantity: Decimal::from_str_exact(qty)?,
                        timestamp: Utc::now(),
                    });
                }
            }
        }
        
        Ok(OrderBook {
            exchange: "OKX".to_string(),
            pair: pair.clone(),
            bids,
            asks,
            timestamp: Utc::now(),
        })
    }
    
    async fn get_fees(&self) -> Result<ExchangeFees> {
        Ok(ExchangeFees {
            maker_fee: Decimal::from_str_exact("0.0008")?, // 0.08%
            taker_fee: Decimal::from_str_exact("0.001")?,  // 0.1%
            withdrawal_fee: vec![
                ("BTC".to_string(), Decimal::from_str_exact("0.0004")?),
                ("ETH".to_string(), Decimal::from_str_exact("0.003")?),
                ("USDT".to_string(), Decimal::from_str_exact("0.8")?),
            ].into_iter().collect(),
        })
    }
    
    async fn get_24h_volume(&self, pair: &TokenPair) -> Result<Decimal> {
        let inst_id = format!("{}-{}", pair.base.symbol, pair.quote.symbol);
        let path = format!("/market/ticker?instId={}", inst_id);
        let response: OkxResponse<OkxTicker> = self.make_request(&path).await?;
        
        if response.data.is_empty() {
            return Ok(Decimal::ZERO);
        }
        
        Ok(Decimal::from_str_exact(&response.data[0].volume_24h)?)
    }
    
    async fn subscribe_to_updates(&self, pairs: Vec<TokenPair>) -> Result<()> {
        self.start_websocket().await;
        Ok(())
    }
}

use rust_decimal::prelude::FromStr;