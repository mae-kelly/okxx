use crate::{config::ExchangeCredentials, types::*};
use super::Exchange;
use anyhow::Result;
use async_trait::async_trait;
use binance::{api::*, market::*, model::*, websockets::*};
use rust_decimal::Decimal;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

pub struct BinanceExchange {
    market: Market,
    credentials: ExchangeCredentials,
    pair_cache: Arc<RwLock<HashMap<String, TokenPair>>>,
    websocket_connections: Arc<RwLock<Vec<WebSockets>>>,
}

impl BinanceExchange {
    pub async fn new(credentials: ExchangeCredentials) -> Result<Self> {
        let market = Binance::new(
            Some(credentials.api_key.clone()),
            Some(credentials.api_secret.clone())
        );

        Ok(Self {
            market,
            credentials,
            pair_cache: Arc::new(RwLock::new(HashMap::new())),
            websocket_connections: Arc::new(RwLock::new(Vec::new())),
        })
    }

    fn convert_symbol_pair(&self, symbol: &str) -> TokenPair {
        let bases = vec!["BTC", "ETH", "BNB", "USDT", "USDC", "BUSD"];
        
        for base in bases {
            if symbol.ends_with(base) {
                let token_symbol = &symbol[..symbol.len() - base.len()];
                return TokenPair {
                    base: Token {
                        address: String::new(),
                        symbol: token_symbol.to_string(),
                        decimals: 18,
                        chain_id: 1,
                    },
                    quote: Token {
                        address: String::new(),
                        symbol: base.to_string(),
                        decimals: 18,
                        chain_id: 1,
                    },
                };
            }
        }

        TokenPair {
            base: Token {
                address: String::new(),
                symbol: symbol[..3].to_string(),
                decimals: 18,
                chain_id: 1,
            },
            quote: Token {
                address: String::new(),
                symbol: symbol[3..].to_string(),
                decimals: 18,
                chain_id: 1,
            },
        }
    }
}

#[async_trait]
impl Exchange for BinanceExchange {
    async fn get_name(&self) -> String {
        "Binance".to_string()
    }

    async fn get_pairs(&self) -> Result<Vec<TokenPair>> {
        let exchange_info = self.market.get_exchange_info()?;
        let mut pairs = Vec::new();

        for symbol in exchange_info.symbols {
            if symbol.status == "TRADING" {
                let pair = TokenPair {
                    base: Token {
                        address: String::new(),
                        symbol: symbol.base_asset,
                        decimals: 18,
                        chain_id: 1,
                    },
                    quote: Token {
                        address: String::new(),
                        symbol: symbol.quote_asset,
                        decimals: 18,
                        chain_id: 1,
                    },
                };
                pairs.push(pair.clone());
                
                let mut cache = self.pair_cache.write().await;
                cache.insert(symbol.symbol, pair);
            }
        }

        Ok(pairs)
    }

    async fn get_price(&self, pair: &TokenPair) -> Result<Price> {
        let symbol = format!("{}{}", pair.base.symbol, pair.quote.symbol);
        let ticker = self.market.get_book_ticker(symbol.clone())?;

        Ok(Price {
            bid: Decimal::from_str_exact(&ticker.bid_price)?,
            ask: Decimal::from_str_exact(&ticker.ask_price)?,
            bid_size: Decimal::from_str_exact(&ticker.bid_qty)?,
            ask_size: Decimal::from_str_exact(&ticker.ask_qty)?,
            timestamp: Utc::now(),
            exchange: "Binance".to_string(),
            pair: pair.clone(),
        })
    }

    async fn get_orderbook(&self, pair: &TokenPair, depth: usize) -> Result<OrderBook> {
        let symbol = format!("{}{}", pair.base.symbol, pair.quote.symbol);
        let orderbook = self.market.get_depth(symbol, Some(depth as u64))?;

        let mut bids = Vec::new();
        let mut asks = Vec::new();

        for bid in orderbook.bids {
            bids.push(Order {
                price: Decimal::from_str_exact(&bid.price)?,
                quantity: Decimal::from_str_exact(&bid.qty)?,
                timestamp: Utc::now(),
            });
        }

        for ask in orderbook.asks {
            asks.push(Order {
                price: Decimal::from_str_exact(&ask.price)?,
                quantity: Decimal::from_str_exact(&ask.qty)?,
                timestamp: Utc::now(),
            });
        }

        Ok(OrderBook {
            exchange: "Binance".to_string(),
            pair: pair.clone(),
            bids,
            asks,
            timestamp: Utc::now(),
        })
    }

    async fn get_fees(&self) -> Result<ExchangeFees> {
        Ok(ExchangeFees {
            maker_fee: Decimal::from_str_exact("0.001")?,
            taker_fee: Decimal::from_str_exact("0.001")?,
            withdrawal_fee: vec![
                ("BTC".to_string(), Decimal::from_str_exact("0.0005")?),
                ("ETH".to_string(), Decimal::from_str_exact("0.005")?),
                ("USDT".to_string(), Decimal::from_str_exact("1.0")?),
            ].into_iter().collect(),
        })
    }

    async fn get_24h_volume(&self, pair: &TokenPair) -> Result<Decimal> {
        let symbol = format!("{}{}", pair.base.symbol, pair.quote.symbol);
        let stats = self.market.get_24h_price_stats(symbol)?;
        
        Ok(Decimal::from_str_exact(&stats.volume)?)
    }

    async fn subscribe_to_updates(&self, pairs: Vec<TokenPair>) -> Result<()> {
        let endpoints: Vec<String> = pairs.iter().map(|pair| {
            let symbol = format!("{}{}", pair.base.symbol.to_lowercase(), pair.quote.symbol.to_lowercase());
            format!("{}@bookTicker", symbol)
        }).collect();

        let ws_endpoint = endpoints.join("/");
        
        let ws = WebSockets::new(|event: WebsocketEvent| {
            match event {
                WebsocketEvent::BookTicker(ticker_event) => {
                    tracing::debug!("Received ticker update for {}", ticker_event.symbol);
                    Ok(())
                },
                _ => Ok(()),
            }
        });

        ws.connect_multiple(&endpoints)?;
        
        let mut connections = self.websocket_connections.write().await;
        connections.push(ws);

        Ok(())
    }
}

use rust_decimal::prelude::FromStr;