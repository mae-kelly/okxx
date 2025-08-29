use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::{json, Value};
use anyhow::Result;
use rust_decimal::Decimal;
use std::str::FromStr;
use chrono::Utc;
use crate::types::{SharedState, PriceData, Chain};

pub struct WebSocketManager {
    state: Arc<SharedState>,
    connections: Vec<WebSocketConnection>,
}

#[derive(Clone)]
struct WebSocketConnection {
    name: String,
    url: String,
    chain: Option<Chain>,
    subscription_message: Value,
}

impl WebSocketManager {
    pub async fn new(state: Arc<SharedState>) -> Result<Self> {
        let connections = vec![
            WebSocketConnection {
                name: "1inch".to_string(),
                url: "wss://api.1inch.io/v5.0/1/ws".to_string(),
                chain: Some(Chain::Ethereum),
                subscription_message: json!({
                    "event": "subscribe",
                    "channel": "quotes",
                    "chainId": 1
                }),
            },
            WebSocketConnection {
                name: "0x".to_string(),
                url: "wss://api.0x.org/ws".to_string(),
                chain: Some(Chain::Ethereum),
                subscription_message: json!({
                    "type": "subscribe",
                    "channel": "orders",
                    "requestId": "1"
                }),
            },
            WebSocketConnection {
                name: "Chainlink".to_string(),
                url: "wss://ws.chain.link/mainnet".to_string(),
                chain: Some(Chain::Ethereum),
                subscription_message: json!({
                    "type": "subscribe",
                    "channel": "prices"
                }),
            },
            WebSocketConnection {
                name: "TheGraph".to_string(),
                url: "wss://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3/graphql".to_string(),
                chain: Some(Chain::Ethereum),
                subscription_message: json!({
                    "type": "connection_init"
                }),
            },
            WebSocketConnection {
                name: "DexScreener".to_string(),
                url: "wss://io.dexscreener.com/dex/screener/pairs/h24/1".to_string(),
                chain: None,
                subscription_message: json!({}),
            },
            WebSocketConnection {
                name: "GeckoTerminal".to_string(),
                url: "wss://api.geckoterminal.com/ws".to_string(),
                chain: None,
                subscription_message: json!({
                    "command": "subscribe",
                    "identifier": json!({
                        "channel": "PoolChannel"
                    }).to_string()
                }),
            },
            WebSocketConnection {
                name: "Binance".to_string(),
                url: "wss://stream.binance.com:9443/ws/!ticker@arr".to_string(),
                chain: None,
                subscription_message: json!({}),
            },
            WebSocketConnection {
                name: "Coinbase".to_string(),
                url: "wss://ws-feed.exchange.coinbase.com".to_string(),
                chain: None,
                subscription_message: json!({
                    "type": "subscribe",
                    "channels": ["ticker", "level2"],
                    "product_ids": ["ETH-USD", "BTC-USD", "MATIC-USD", "ARB-USD", "OP-USD"]
                }),
            },
            WebSocketConnection {
                name: "Kraken".to_string(),
                url: "wss://ws.kraken.com".to_string(),
                chain: None,
                subscription_message: json!({
                    "event": "subscribe",
                    "pair": ["ETH/USD", "BTC/USD"],
                    "subscription": {
                        "name": "ticker"
                    }
                }),
            },
            WebSocketConnection {
                name: "Bitfinex".to_string(),
                url: "wss://api-pub.bitfinex.com/ws/2".to_string(),
                chain: None,
                subscription_message: json!({
                    "event": "subscribe",
                    "channel": "ticker",
                    "symbol": "tETHUSD"
                }),
            },
            WebSocketConnection {
                name: "OKX".to_string(),
                url: "wss://ws.okx.com:8443/ws/v5/public".to_string(),
                chain: None,
                subscription_message: json!({
                    "op": "subscribe",
                    "args": [{
                        "channel": "tickers",
                        "instId": "ETH-USDT"
                    }]
                }),
            },
            WebSocketConnection {
                name: "Bybit".to_string(),
                url: "wss://stream.bybit.com/v5/public/spot".to_string(),
                chain: None,
                subscription_message: json!({
                    "op": "subscribe",
                    "args": ["orderbook.50.ETHUSDT", "publicTrade.ETHUSDT"]
                }),
            },
            WebSocketConnection {
                name: "Gate.io".to_string(),
                url: "wss://api.gateio.ws/ws/v4/".to_string(),
                chain: None,
                subscription_message: json!({
                    "channel": "spot.tickers",
                    "event": "subscribe",
                    "payload": ["ETH_USDT", "BTC_USDT"]
                }),
            },
            WebSocketConnection {
                name: "KuCoin".to_string(),
                url: "wss://ws-api-spot.kucoin.com".to_string(),
                chain: None,
                subscription_message: json!({
                    "type": "subscribe",
                    "topic": "/market/ticker:ETH-USDT,BTC-USDT"
                }),
            },
            WebSocketConnection {
                name: "MEXC".to_string(),
                url: "wss://wbs.mexc.com/ws".to_string(),
                chain: None,
                subscription_message: json!({
                    "method": "SUBSCRIPTION",
                    "params": ["spot@public.deals.v3.api@ETHUSDT", "spot@public.bookTicker.v3.api@ETHUSDT"]
                }),
            },
        ];
        
        Ok(Self {
            state,
            connections,
        })
    }
    
    pub async fn start_all_connections(&self) {
        for conn in &self.connections {
            let conn_clone = WebSocketConnection {
                name: conn.name.clone(),
                url: conn.url.clone(),
                chain: conn.chain.clone(),
                subscription_message: conn.subscription_message.clone(),
            };
            let state_clone = self.state.clone();
            
            tokio::spawn(async move {
                loop {
                    if let Err(e) = Self::connect_and_listen(conn_clone.clone(), state_clone.clone()).await {
                        tracing::error!("WebSocket {} error: {}", conn_clone.name, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            });
        }
    }
    
    async fn connect_and_listen(conn: WebSocketConnection, state: Arc<SharedState>) -> Result<()> {
        let url = url::Url::parse(&conn.url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        if !conn.subscription_message.is_null() {
            write.send(Message::Text(conn.subscription_message.to_string())).await?;
        }
        
        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        Self::process_message(&conn.name, data, &state, conn.chain.as_ref()).await;
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if let Ok(text) = String::from_utf8(bin) {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            Self::process_message(&conn.name, data, &state, conn.chain.as_ref()).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("WebSocket {} read error: {}", conn.name, e);
                    break;
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn process_message(source: &str, data: Value, state: &Arc<SharedState>, chain: Option<&Chain>) {
        match source {
            "Binance" => Self::process_binance_message(data, state).await,
            "Coinbase" => Self::process_coinbase_message(data, state).await,
            "Kraken" => Self::process_kraken_message(data, state).await,
            "DexScreener" => Self::process_dexscreener_message(data, state).await,
            "1inch" => Self::process_1inch_message(data, state, chain).await,
            "0x" => Self::process_0x_message(data, state, chain).await,
            _ => {}
        }
    }
    
    async fn process_binance_message(data: Value, state: &Arc<SharedState>) {
        if let Some(arr) = data.as_array() {
            for item in arr {
                if let (Some(symbol), Some(price), Some(volume)) = (
                    item["s"].as_str(),
                    item["c"].as_str(),
                    item["v"].as_str(),
                ) {
                    if let (Ok(price_dec), Ok(vol_dec)) = (
                        Decimal::from_str(price),
                        Decimal::from_str(volume),
                    ) {
                        let price_data = PriceData {
                            token_pair: symbol.to_string(),
                            price: price_dec,
                            volume_24h: vol_dec,
                            liquidity: vol_dec * price_dec,
                            exchange: "Binance".to_string(),
                            chain: Chain::BinanceSmartChain,
                            timestamp: Utc::now(),
                            bid: price_dec,
                            ask: price_dec,
                            spread: Decimal::ZERO,
                        };
                        
                        let key = format!("binance_{}", symbol);
                        state.prices.insert(key, price_data);
                    }
                }
            }
        }
    }
    
    async fn process_coinbase_message(data: Value, state: &Arc<SharedState>) {
        if data["type"] == "ticker" {
            if let (Some(product_id), Some(price), Some(volume)) = (
                data["product_id"].as_str(),
                data["price"].as_str(),
                data["volume_24h"].as_str(),
            ) {
                if let (Ok(price_dec), Ok(vol_dec)) = (
                    Decimal::from_str(price),
                    Decimal::from_str(volume),
                ) {
                    let price_data = PriceData {
                        token_pair: product_id.to_string(),
                        price: price_dec,
                        volume_24h: vol_dec,
                        liquidity: vol_dec * price_dec,
                        exchange: "Coinbase".to_string(),
                        chain: Chain::Ethereum,
                        timestamp: Utc::now(),
                        bid: price_dec,
                        ask: price_dec,
                        spread: Decimal::ZERO,
                    };
                    
                    let key = format!("coinbase_{}", product_id);
                    state.prices.insert(key, price_data);
                }
            }
        }
    }
    
    async fn process_kraken_message(data: Value, state: &Arc<SharedState>) {
        if let Some(channel_data) = data.as_array() {
            if channel_data.len() >= 4 {
                if let (Some(pair), Some(ticker_data)) = (
                    channel_data[3].as_str(),
                    channel_data[1].as_object(),
                ) {
                    if let (Some(ask), Some(bid), Some(volume)) = (
                        ticker_data["a"].as_array().and_then(|a| a[0].as_str()),
                        ticker_data["b"].as_array().and_then(|b| b[0].as_str()),
                        ticker_data["v"].as_array().and_then(|v| v[1].as_str()),
                    ) {
                        if let (Ok(ask_dec), Ok(bid_dec), Ok(vol_dec)) = (
                            Decimal::from_str(ask),
                            Decimal::from_str(bid),
                            Decimal::from_str(volume),
                        ) {
                            let price = (ask_dec + bid_dec) / Decimal::from(2);
                            let price_data = PriceData {
                                token_pair: pair.to_string(),
                                price,
                                volume_24h: vol_dec,
                                liquidity: vol_dec * price,
                                exchange: "Kraken".to_string(),
                                chain: Chain::Ethereum,
                                timestamp: Utc::now(),
                                bid: bid_dec,
                                ask: ask_dec,
                                spread: ask_dec - bid_dec,
                            };
                            
                            let key = format!("kraken_{}", pair);
                            state.prices.insert(key, price_data);
                        }
                    }
                }
            }
        }
    }
    
    async fn process_dexscreener_message(data: Value, state: &Arc<SharedState>) {
        if let Some(pairs) = data["pairs"].as_array() {
            for pair in pairs {
                if let (Some(pair_address), Some(price), Some(liquidity)) = (
                    pair["pairAddress"].as_str(),
                    pair["priceUsd"].as_str(),
                    pair["liquidity"]["usd"].as_f64(),
                ) {
                    if let Ok(price_dec) = Decimal::from_str(price) {
                        let chain = match pair["chainId"].as_str() {
                            Some("ethereum") => Chain::Ethereum,
                            Some("bsc") => Chain::BinanceSmartChain,
                            Some("polygon") => Chain::Polygon,
                            Some("arbitrum") => Chain::Arbitrum,
                            Some("optimism") => Chain::Optimism,
                            Some("avalanche") => Chain::Avalanche,
                            Some("fantom") => Chain::Fantom,
                            _ => Chain::Ethereum,
                        };
                        
                        let price_data = PriceData {
                            token_pair: pair_address.to_string(),
                            price: price_dec,
                            volume_24h: Decimal::from_f64_retain(pair["volume"]["h24"].as_f64().unwrap_or(0.0)).unwrap_or(Decimal::ZERO),
                            liquidity: Decimal::from_f64_retain(liquidity).unwrap_or(Decimal::ZERO),
                            exchange: "DexScreener".to_string(),
                            chain,
                            timestamp: Utc::now(),
                            bid: price_dec,
                            ask: price_dec,
                            spread: Decimal::ZERO,
                        };
                        
                        let key = format!("dexscreener_{}", pair_address);
                        state.prices.insert(key, price_data);
                    }
                }
            }
        }
    }
    
    async fn process_1inch_message(data: Value, state: &Arc<SharedState>, chain: Option<&Chain>) {
        if let Some(quote) = data["quote"].as_object() {
            if let (Some(from_token), Some(to_token), Some(from_amount), Some(to_amount)) = (
                quote["fromToken"].as_str(),
                quote["toToken"].as_str(),
                quote["fromTokenAmount"].as_str(),
                quote["toTokenAmount"].as_str(),
            ) {
                if let (Ok(from_amt), Ok(to_amt)) = (
                    Decimal::from_str(from_amount),
                    Decimal::from_str(to_amount),
                ) {
                    let price = to_amt / from_amt;
                    let price_data = PriceData {
                        token_pair: format!("{}/{}", from_token, to_token),
                        price,
                        volume_24h: Decimal::ZERO,
                        liquidity: Decimal::ZERO,
                        exchange: "1inch".to_string(),
                        chain: chain.cloned().unwrap_or(Chain::Ethereum),
                        timestamp: Utc::now(),
                        bid: price,
                        ask: price,
                        spread: Decimal::ZERO,
                    };
                    
                    let key = format!("1inch_{}_{}", from_token, to_token);
                    state.prices.insert(key, price_data);
                }
            }
        }
    }
    
    async fn process_0x_message(data: Value, state: &Arc<SharedState>, chain: Option<&Chain>) {
        if let Some(order) = data["order"].as_object() {
            if let (Some(maker_token), Some(taker_token), Some(maker_amount), Some(taker_amount)) = (
                order["makerToken"].as_str(),
                order["takerToken"].as_str(),
                order["makerAmount"].as_str(),
                order["takerAmount"].as_str(),
            ) {
                if let (Ok(maker_amt), Ok(taker_amt)) = (
                    Decimal::from_str(maker_amount),
                    Decimal::from_str(taker_amount),
                ) {
                    let price = taker_amt / maker_amt;
                    let price_data = PriceData {
                        token_pair: format!("{}/{}", maker_token, taker_token),
                        price,
                        volume_24h: Decimal::ZERO,
                        liquidity: Decimal::ZERO,
                        exchange: "0x".to_string(),
                        chain: chain.cloned().unwrap_or(Chain::Ethereum),
                        timestamp: Utc::now(),
                        bid: price,
                        ask: price,
                        spread: Decimal::ZERO,
                    };
                    
                    let key = format!("0x_{}_{}", maker_token, taker_token);
                    state.prices.insert(key, price_data);
                }
            }
        }
    }
}