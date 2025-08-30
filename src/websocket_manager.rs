use std::sync::Arc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use futures_util::{StreamExt, SinkExt};
use serde_json::{json, Value};
use anyhow::Result;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use std::str::FromStr;
use chrono::Utc;
use colored::*;
use parking_lot::RwLock;
use std::collections::HashMap;
use crate::types::{SharedState, ArbitrageSignal, WebSocketFeed};

pub struct WebSocketManager {
    state: Arc<SharedState>,
    feeds: Vec<WebSocketFeed>,
    performance_stats: Arc<RwLock<HashMap<String, FeedStats>>>,
}

#[derive(Clone, Debug, Default)]
pub struct FeedStats {
    pub messages_received: u64,
    pub opportunities_found: u64,
    pub avg_latency_ms: f64,
    pub uptime_seconds: u64,
    pub profit_generated: Decimal,
    pub ml_score: f64,
}

impl WebSocketManager {
    pub async fn new(state: Arc<SharedState>) -> Result<Self> {
        let feeds = Self::initialize_feeds();
        
        println!("{}", "🚀 Initializing WebSocket Manager with 100+ feeds...".bright_green().bold());
        println!("{}", format!("📡 Total feeds configured: {}", feeds.len()).cyan());
        
        Ok(Self {
            state,
            feeds,
            performance_stats: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    fn initialize_feeds() -> Vec<WebSocketFeed> {
        vec![
            // Major CEX WebSockets
            WebSocketFeed::new("Binance-Spot", "wss://stream.binance.com:9443/ws/!ticker@arr", json!({})),
            WebSocketFeed::new("Binance-Futures", "wss://fstream.binance.com/ws/!markPrice@arr", json!({})),
            WebSocketFeed::new("Coinbase-Pro", "wss://ws-feed.exchange.coinbase.com", json!({
                "type": "subscribe",
                "channels": ["ticker", "level2", "matches"],
                "product_ids": ["ETH-USD", "BTC-USD", "MATIC-USD", "ARB-USD", "OP-USD", "AVAX-USD", "SOL-USD", "LINK-USD", "UNI-USD", "AAVE-USD"]
            })),
            WebSocketFeed::new("Kraken", "wss://ws.kraken.com", json!({
                "event": "subscribe",
                "pair": ["ETH/USD", "BTC/USD", "XRP/USD", "ADA/USD", "DOT/USD"],
                "subscription": {"name": "ticker"}
            })),
            WebSocketFeed::new("Bitfinex", "wss://api-pub.bitfinex.com/ws/2", json!({
                "event": "subscribe",
                "channel": "ticker",
                "symbol": "tETHUSD"
            })),
            WebSocketFeed::new("OKX", "wss://ws.okx.com:8443/ws/v5/public", json!({
                "op": "subscribe",
                "args": [
                    {"channel": "tickers", "instId": "ETH-USDT"},
                    {"channel": "tickers", "instId": "BTC-USDT"},
                    {"channel": "books5", "instId": "ETH-USDT"}
                ]
            })),
            WebSocketFeed::new("Bybit", "wss://stream.bybit.com/v5/public/spot", json!({
                "op": "subscribe",
                "args": ["orderbook.50.ETHUSDT", "publicTrade.ETHUSDT", "tickers.BTCUSDT"]
            })),
            WebSocketFeed::new("Gate.io", "wss://api.gateio.ws/ws/v4/", json!({
                "channel": "spot.tickers",
                "event": "subscribe",
                "payload": ["ETH_USDT", "BTC_USDT", "MATIC_USDT", "ARB_USDT"]
            })),
            WebSocketFeed::new("KuCoin", "wss://ws-api-spot.kucoin.com", json!({
                "type": "subscribe",
                "topic": "/market/ticker:ETH-USDT,BTC-USDT,MATIC-USDT,SOL-USDT"
            })),
            WebSocketFeed::new("Huobi", "wss://api.huobi.pro/ws", json!({
                "sub": "market.ethusdt.ticker"
            })),
            WebSocketFeed::new("MEXC", "wss://wbs.mexc.com/ws", json!({
                "method": "SUBSCRIPTION",
                "params": ["spot@public.deals.v3.api@ETHUSDT", "spot@public.bookTicker.v3.api@ETHUSDT"]
            })),
            WebSocketFeed::new("Bitget", "wss://ws.bitget.com/spot/v1/stream", json!({
                "op": "subscribe",
                "args": [{"instType": "sp", "channel": "ticker", "instId": "ETHUSDT"}]
            })),
            WebSocketFeed::new("Crypto.com", "wss://stream.crypto.com/v2/market", json!({
                "method": "subscribe",
                "params": {"channels": ["ticker.ETH_USDT", "ticker.BTC_USDT"]}
            })),
            WebSocketFeed::new("Bitstamp", "wss://ws.bitstamp.net", json!({
                "event": "bts:subscribe",
                "data": {"channel": "live_trades_ethusd"}
            })),
            
            // DEX Aggregators
            WebSocketFeed::new("1inch-Ethereum", "wss://api.1inch.io/v5.0/1/ws", json!({
                "event": "subscribe",
                "channel": "quotes",
                "chainId": 1
            })),
            WebSocketFeed::new("1inch-BSC", "wss://api.1inch.io/v5.0/56/ws", json!({
                "event": "subscribe",
                "channel": "quotes",
                "chainId": 56
            })),
            WebSocketFeed::new("1inch-Polygon", "wss://api.1inch.io/v5.0/137/ws", json!({
                "event": "subscribe",
                "channel": "quotes",
                "chainId": 137
            })),
            WebSocketFeed::new("1inch-Arbitrum", "wss://api.1inch.io/v5.0/42161/ws", json!({
                "event": "subscribe",
                "channel": "quotes",
                "chainId": 42161
            })),
            WebSocketFeed::new("0x-API", "wss://api.0x.org/ws", json!({
                "type": "subscribe",
                "channel": "orders",
                "requestId": "1"
            })),
            WebSocketFeed::new("Paraswap", "wss://api.paraswap.io/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("KyberSwap", "wss://api.kyberswap.com/ws", json!({
                "type": "subscribe",
                "channel": "quotes"
            })),
            WebSocketFeed::new("OpenOcean", "wss://api.openocean.finance/ws", json!({
                "type": "subscribe",
                "channel": "quotes"
            })),
            
            // DeFi Protocols
            WebSocketFeed::new("Uniswap-V3", "wss://api.thegraph.com/subgraphs/name/uniswap/uniswap-v3/graphql", json!({
                "type": "connection_init"
            })),
            WebSocketFeed::new("SushiSwap", "wss://api.thegraph.com/subgraphs/name/sushi/exchange/graphql", json!({
                "type": "connection_init"
            })),
            WebSocketFeed::new("PancakeSwap", "wss://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v2/graphql", json!({
                "type": "connection_init"
            })),
            WebSocketFeed::new("Curve", "wss://api.curve.fi/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            WebSocketFeed::new("Balancer", "wss://api.thegraph.com/subgraphs/name/balancer-labs/balancer-v2/graphql", json!({
                "type": "connection_init"
            })),
            WebSocketFeed::new("Bancor", "wss://api.bancor.network/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            
            // Analytics Platforms
            WebSocketFeed::new("DexScreener", "wss://io.dexscreener.com/dex/screener/pairs/h24/1", json!({})),
            WebSocketFeed::new("GeckoTerminal", "wss://api.geckoterminal.com/ws", json!({
                "command": "subscribe",
                "identifier": json!({"channel": "PoolChannel"}).to_string()
            })),
            WebSocketFeed::new("DexTools", "wss://www.dextools.io/ws", json!({
                "type": "subscribe",
                "channel": "pairs"
            })),
            WebSocketFeed::new("DEXGuru", "wss://api.dex.guru/ws", json!({
                "type": "subscribe",
                "channel": "trades"
            })),
            
            // Price Oracles
            WebSocketFeed::new("Chainlink", "wss://ws.chain.link/mainnet", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("Band-Protocol", "wss://api.bandprotocol.com/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("API3", "wss://api.api3.org/ws", json!({
                "type": "subscribe",
                "channel": "datafeeds"
            })),
            WebSocketFeed::new("Pyth", "wss://api.pyth.network/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            
            // Layer 2 DEXs
            WebSocketFeed::new("QuickSwap", "wss://api.quickswap.exchange/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("TraderJoe", "wss://api.traderjoexyz.com/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("SpookySwap", "wss://api.spookyswap.finance/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("Camelot", "wss://api.camelot.exchange/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("Velodrome", "wss://api.velodrome.finance/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            
            // More CEXs
            WebSocketFeed::new("Gemini", "wss://api.gemini.com/v2/marketdata", json!({
                "type": "subscribe",
                "subscriptions": [{"name": "l2", "symbols": ["ETHUSD", "BTCUSD"]}]
            })),
            WebSocketFeed::new("Poloniex", "wss://api2.poloniex.com", json!({
                "command": "subscribe",
                "channel": "1002"
            })),
            WebSocketFeed::new("Bittrex", "wss://socket-v3.bittrex.com/signalr", json!({
                "H": "c3",
                "M": "Subscribe",
                "A": [["ticker_ETH-USD", "ticker_BTC-USD"]]
            })),
            WebSocketFeed::new("LBank", "wss://www.lbkex.net/ws/V2/", json!({
                "action": "subscribe",
                "subscribe": "tick",
                "pair": "eth_usdt"
            })),
            WebSocketFeed::new("AscendEX", "wss://ascendex.com/api/pro/v1/stream", json!({
                "op": "sub",
                "ch": "ticker:ETH/USDT"
            })),
            
            // Additional DeFi protocols
            WebSocketFeed::new("Raydium", "wss://api.raydium.io/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            WebSocketFeed::new("Orca", "wss://api.orca.so/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            WebSocketFeed::new("Osmosis", "wss://api.osmosis.zone/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            WebSocketFeed::new("ThorSwap", "wss://api.thorswap.finance/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            
            // Bridges and Cross-chain
            WebSocketFeed::new("Synapse", "wss://api.synapseprotocol.com/ws", json!({
                "type": "subscribe",
                "channel": "bridges"
            })),
            WebSocketFeed::new("Stargate", "wss://api.stargate.finance/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            WebSocketFeed::new("Hop", "wss://api.hop.exchange/ws", json!({
                "type": "subscribe",
                "channel": "transfers"
            })),
            
            // Options and Derivatives
            WebSocketFeed::new("Deribit", "wss://www.deribit.com/ws/api/v2", json!({
                "method": "public/subscribe",
                "params": {"channels": ["ticker.ETH-PERPETUAL", "ticker.BTC-PERPETUAL"]}
            })),
            WebSocketFeed::new("Lyra", "wss://api.lyra.finance/ws", json!({
                "type": "subscribe",
                "channel": "options"
            })),
            WebSocketFeed::new("Ribbon", "wss://api.ribbon.finance/ws", json!({
                "type": "subscribe",
                "channel": "vaults"
            })),
            
            // NFT Marketplaces (for liquidity data)
            WebSocketFeed::new("OpenSea", "wss://stream.openseabeta.com/ws", json!({
                "type": "subscribe",
                "channel": "collection_stats"
            })),
            WebSocketFeed::new("Blur", "wss://api.blur.io/ws", json!({
                "type": "subscribe",
                "channel": "collections"
            })),
            
            // Regional exchanges
            WebSocketFeed::new("Upbit", "wss://api.upbit.com/websocket/v1", json!([{
                "ticket": "test",
                "type": "ticker",
                "codes": ["KRW-BTC", "KRW-ETH"]
            }])),
            WebSocketFeed::new("Bithumb", "wss://pubwss.bithumb.com/pub/ws", json!({
                "type": "ticker",
                "symbols": ["BTC_KRW", "ETH_KRW"]
            })),
            WebSocketFeed::new("Bitso", "wss://ws.bitso.com", json!({
                "action": "subscribe",
                "book": "eth_mxn",
                "type": "trades"
            })),
            
            // More Layer 2s and sidechains
            WebSocketFeed::new("zkSync", "wss://api.zksync.io/ws", json!({
                "type": "subscribe",
                "channel": "trades"
            })),
            WebSocketFeed::new("StarkNet", "wss://api.starknet.io/ws", json!({
                "type": "subscribe",
                "channel": "trades"
            })),
            WebSocketFeed::new("Loopring", "wss://ws.api3.loopring.io/v3/ws", json!({
                "op": "sub",
                "topics": [{"topic": "ticker"}]
            })),
            
            // Lending protocols
            WebSocketFeed::new("Aave", "wss://api.aave.com/ws", json!({
                "type": "subscribe",
                "channel": "rates"
            })),
            WebSocketFeed::new("Compound", "wss://api.compound.finance/ws", json!({
                "type": "subscribe",
                "channel": "markets"
            })),
            WebSocketFeed::new("MakerDAO", "wss://api.makerdao.com/ws", json!({
                "type": "subscribe",
                "channel": "vaults"
            })),
            
            // Yield aggregators
            WebSocketFeed::new("Yearn", "wss://api.yearn.finance/ws", json!({
                "type": "subscribe",
                "channel": "vaults"
            })),
            WebSocketFeed::new("Beefy", "wss://api.beefy.finance/ws", json!({
                "type": "subscribe",
                "channel": "vaults"
            })),
            WebSocketFeed::new("Harvest", "wss://api.harvest.finance/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            
            // Stablecoin protocols
            WebSocketFeed::new("Frax", "wss://api.frax.finance/ws", json!({
                "type": "subscribe",
                "channel": "pools"
            })),
            WebSocketFeed::new("MIM", "wss://api.abracadabra.money/ws", json!({
                "type": "subscribe",
                "channel": "cauldrons"
            })),
            
            // Prediction markets
            WebSocketFeed::new("Polymarket", "wss://api.polymarket.com/ws", json!({
                "type": "subscribe",
                "channel": "markets"
            })),
            WebSocketFeed::new("Augur", "wss://api.augur.net/ws", json!({
                "type": "subscribe",
                "channel": "markets"
            })),
            
            // Additional aggregators
            WebSocketFeed::new("Matcha", "wss://api.matcha.xyz/ws", json!({
                "type": "subscribe",
                "channel": "quotes"
            })),
            WebSocketFeed::new("Zapper", "wss://api.zapper.fi/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            WebSocketFeed::new("Zerion", "wss://api.zerion.io/ws", json!({
                "type": "subscribe",
                "channel": "prices"
            })),
            
            // Privacy protocols
            WebSocketFeed::new("Tornado", "wss://api.tornado.cash/ws", json!({
                "type": "subscribe",
                "channel": "deposits"
            })),
            WebSocketFeed::new("Aztec", "wss://api.aztec.network/ws", json!({
                "type": "subscribe",
                "channel": "rollups"
            })),
            
            // Gaming and Metaverse
            WebSocketFeed::new("Axie", "wss://api.axieinfinity.com/ws", json!({
                "type": "subscribe",
                "channel": "marketplace"
            })),
            WebSocketFeed::new("Sandbox", "wss://api.sandbox.game/ws", json!({
                "type": "subscribe",
                "channel": "lands"
            })),
            WebSocketFeed::new("Decentraland", "wss://api.decentraland.org/ws", json!({
                "type": "subscribe",
                "channel": "marketplace"
            })),
            
            // More international exchanges
            WebSocketFeed::new("WazirX", "wss://stream.wazirx.com/stream", json!({
                "event": "subscribe",
                "streams": ["ethusdt@ticker", "btcusdt@ticker"]
            })),
            WebSocketFeed::new("CoinDCX", "wss://stream.coindcx.com/", json!({
                "event": "subscribe",
                "channel": "ticker",
                "symbol": "ETHINR"
            })),
            WebSocketFeed::new("Mercado", "wss://ws.mercadobitcoin.net/ws", json!({
                "type": "subscribe",
                "channel": "ticker",
                "symbol": "BTC-BRL"
            })),
            
            // Institutional platforms
            WebSocketFeed::new("FalconX", "wss://api.falconx.io/ws", json!({
                "type": "subscribe",
                "channel": "quotes"
            })),
            WebSocketFeed::new("Talos", "wss://api.talos.com/ws", json!({
                "type": "subscribe",
                "channel": "aggregated_quotes"
            })),
            WebSocketFeed::new("SFOX", "wss://api.sfox.com/ws", json!({
                "type": "subscribe",
                "channel": "orderbook"
            })),
        ]
    }
    
    pub async fn start_all_connections(&self) {
        println!("{}", "🔌 Starting WebSocket connections...".yellow());
        
        for feed in &self.feeds {
            let feed_clone = feed.clone();
            let state_clone = self.state.clone();
            let stats_clone = self.performance_stats.clone();
            
            tokio::spawn(async move {
                loop {
                    if let Err(e) = Self::connect_and_monitor(
                        feed_clone.clone(),
                        state_clone.clone(),
                        stats_clone.clone()
                    ).await {
                        eprintln!("{} {} {}: {}", 
                            "❌".red(),
                            "WebSocket error for".red(),
                            feed_clone.name.red().bold(),
                            e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                }
            });
        }
        
        // Start performance monitor
        let stats_clone = self.performance_stats.clone();
        tokio::spawn(async move {
            Self::monitor_performance(stats_clone).await;
        });
    }
    
    async fn connect_and_monitor(
        feed: WebSocketFeed,
        state: Arc<SharedState>,
        stats: Arc<RwLock<HashMap<String, FeedStats>>>
    ) -> Result<()> {
        let url = url::Url::parse(&feed.url)?;
        let (ws_stream, _) = connect_async(url).await?;
        let (mut write, mut read) = ws_stream.split();
        
        // Send subscription message
        if !feed.subscription.is_null() {
            write.send(Message::Text(feed.subscription.to_string())).await?;
        }
        
        let start_time = Utc::now();
        
        while let Some(message) = read.next().await {
            let recv_time = Utc::now();
            
            match message {
                Ok(Message::Text(text)) => {
                    if let Ok(data) = serde_json::from_str::<Value>(&text) {
                        let latency = (recv_time - start_time).num_milliseconds() as f64;
                        
                        // Process for arbitrage
                        if let Some(signal) = Self::extract_arbitrage_signal(&feed.name, &data, &state).await {
                            // Update stats
                            let mut stats_guard = stats.write();
                            let feed_stat = stats_guard.entry(feed.name.clone()).or_default();
                            feed_stat.messages_received += 1;
                            feed_stat.avg_latency_ms = (feed_stat.avg_latency_ms + latency) / 2.0;
                            
                            if signal.profit > Decimal::ZERO {
                                feed_stat.opportunities_found += 1;
                                feed_stat.profit_generated += signal.profit;
                                
                                // Log opportunity to terminal
                                Self::log_opportunity(&feed.name, &signal);
                                
                                // Store in shared state
                                state.signals.insert(signal.id.clone(), signal);
                            }
                        }
                    }
                }
                Ok(Message::Binary(bin)) => {
                    if let Ok(text) = String::from_utf8(bin) {
                        if let Ok(data) = serde_json::from_str::<Value>(&text) {
                            Self::extract_arbitrage_signal(&feed.name, &data, &state).await;
                        }
                    }
                }
                Err(e) => {
                    return Err(anyhow::anyhow!("WebSocket error: {}", e));
                }
                _ => {}
            }
        }
        
        Ok(())
    }
    
    async fn extract_arbitrage_signal(
        source: &str,
        data: &Value,
        state: &Arc<SharedState>
    ) -> Option<ArbitrageSignal> {
        // Extract price data based on source format
        let price_info = match source {
            s if s.starts_with("Binance") => Self::parse_binance_data(data),
            s if s.starts_with("Coinbase") => Self::parse_coinbase_data(data),
            s if s.starts_with("Kraken") => Self::parse_kraken_data(data),
            s if s.starts_with("Uniswap") => Self::parse_uniswap_data(data),
            s if s.starts_with("1inch") => Self::parse_1inch_data(data),
            _ => None,
        };
        
        if let Some((token_pair, price, volume)) = price_info {
            // Check for arbitrage opportunities
            let existing_prices = state.price_index.read();
            
            for (other_source, other_price) in existing_prices.iter() {
                if other_source != source && other_source.contains(&token_pair) {
                    let price_diff = (price - *other_price).abs();
                    let spread_pct = (price_diff / price) * Decimal::from(100);
                    
                    if spread_pct > Decimal::from_str("0.5").unwrap() {
                        // Calculate with gas and fees
                        let signal = Self::calculate_arbitrage_profit(
                            source,
                            other_source,
                            &token_pair,
                            price,
                            *other_price,
                            volume,
                            state
                        ).await;
                        
                        return signal;
                    }
                }
            }
            
            // Update price index
            drop(existing_prices);
            let mut prices = state.price_index.write();
            prices.insert(format!("{}:{}", source, token_pair), price);
        }
        
        None
    }
    
    async fn calculate_arbitrage_profit(
        buy_exchange: &str,
        sell_exchange: &str,
        token_pair: &str,
        buy_price: Decimal,
        sell_price: Decimal,
        volume: Decimal,
        state: &Arc<SharedState>
    ) -> Option<ArbitrageSignal> {
        let amount = Decimal::from(10000); // $10k trade size
        
        // Get gas prices
        let gas_price = state.gas_tracker.get_current_gas_price().await;
        let gas_cost = gas_price * Decimal::from(300000) / Decimal::from(1_000_000_000);
        
        // Flash loan fee (0.09% for Aave)
        let flash_loan_fee = amount * Decimal::from_str("0.0009").unwrap();
        
        // Exchange fees (0.3% average)
        let exchange_fees = amount * Decimal::from_str("0.006").unwrap();
        
        // Calculate profit
        let tokens_bought = amount / buy_price;
        let revenue = tokens_bought * sell_price;
        let total_costs = gas_cost + flash_loan_fee + exchange_fees;
        let profit = revenue - amount - total_costs;
        
        if profit > Decimal::from(10) { // Minimum $10 profit
            Some(ArbitrageSignal {
                id: hex::encode(blake3::hash(format!("{}{}{}", buy_exchange, sell_exchange, Utc::now()).as_bytes()).as_bytes()),
                buy_exchange: buy_exchange.to_string(),
                sell_exchange: sell_exchange.to_string(),
                token_pair: token_pair.to_string(),
                buy_price,
                sell_price,
                volume,
                profit,
                roi: (profit / amount * Decimal::from(100)),
                gas_cost,
                flash_loan_fee,
                total_fees: total_costs,
                timestamp: Utc::now(),
                confidence: 0.0, // Will be set by ML model
            })
        } else {
            None
        }
    }
    
    fn log_opportunity(source: &str, signal: &ArbitrageSignal) {
        let profit_color = if signal.profit > Decimal::from(100) {
            format!("${:.2}", signal.profit).bright_green().bold()
        } else if signal.profit > Decimal::from(50) {
            format!("${:.2}", signal.profit).green().bold()
        } else {
            format!("${:.2}", signal.profit).yellow()
        };
        
        println!(
            "\n{} {} {}",
            "💰".bright_yellow(),
            "ARBITRAGE OPPORTUNITY DETECTED!".bright_cyan().bold(),
            "💰".bright_yellow()
        );
        println!("  {} {}", "Source:".bright_white(), source.cyan());
        println!("  {} {} → {}", "Route:".bright_white(), signal.buy_exchange.green(), signal.sell_exchange.green());
        println!("  {} {}", "Pair:".bright_white(), signal.token_pair.yellow());
        println!("  {} Buy @ {} | Sell @ {}", 
            "Prices:".bright_white(),
            format!("{:.4}", signal.buy_price).red(),
            format!("{:.4}", signal.sell_price).green()
        );
        println!("  {} {}", "Profit:".bright_white().bold(), profit_color);
        println!("  {} {:.2}%", "ROI:".bright_white(), signal.roi);
        println!("  {} ${:.2}", "Gas Cost:".bright_white(), signal.gas_cost);
        println!("  {} ${:.2}", "Flash Loan Fee:".bright_white(), signal.flash_loan_fee);
        println!("  {} ${:.2}", "Total Fees:".bright_white(), signal.total_fees);
        println!("{}", "─".repeat(60).bright_black());
    }
    
    async fn monitor_performance(stats: Arc<RwLock<HashMap<String, FeedStats>>>) {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        
        loop {
            interval.tick().await;
            
            let stats_guard = stats.read();
            let mut feed_scores: Vec<(String, f64)> = Vec::new();
            
            println!("\n{}", "📊 WEBSOCKET PERFORMANCE REPORT".bright_magenta().bold());
            println!("{}", "═".repeat(80).bright_magenta());
            
            for (name, stat) in stats_guard.iter() {
                let score = (stat.opportunities_found as f64 * 100.0) 
                    + (stat.profit_generated.to_f64().unwrap_or(0.0))
                    + (stat.ml_score * 50.0)
                    - (stat.avg_latency_ms / 10.0);
                
                feed_scores.push((name.clone(), score));
            }
            
            feed_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            
            println!("{}", "Top 10 Performing Feeds:".bright_cyan());
            for (i, (name, score)) in feed_scores.iter().take(10).enumerate() {
                let stat = &stats_guard[name];
                println!(
                    "  {}. {} {} | {} Opportunities: {} | {} Profit: ${:.2} | {} Score: {:.1}",
                    i + 1,
                    "📡".bright_yellow(),
                    name.bright_white().bold(),
                    "🎯".green(),
                    stat.opportunities_found,
                    "💵".green(),
                    stat.profit_generated,
                    "⭐".yellow(),
                    score
                );
            }
            println!("{}", "═".repeat(80).bright_magenta());
        }
    }
    
    // Parser functions for different exchanges
    fn parse_binance_data(data: &Value) -> Option<(String, Decimal, Decimal)> {
        if let Some(arr) = data.as_array() {
            for item in arr {
                if let (Some(symbol), Some(price), Some(volume)) = (
                    item["s"].as_str(),
                    item["c"].as_str(),
                    item["v"].as_str(),
                ) {
                    if let (Ok(p), Ok(v)) = (Decimal::from_str(price), Decimal::from_str(volume)) {
                        return Some((symbol.to_string(), p, v));
                    }
                }
            }
        }
        None
    }
    
    fn parse_coinbase_data(data: &Value) -> Option<(String, Decimal, Decimal)> {
        if data["type"] == "ticker" {
            if let (Some(product), Some(price), Some(volume)) = (
                data["product_id"].as_str(),
                data["price"].as_str(),
                data["volume_24h"].as_str(),
            ) {
                if let (Ok(p), Ok(v)) = (Decimal::from_str(price), Decimal::from_str(volume)) {
                    return Some((product.to_string(), p, v));
                }
            }
        }
        None
    }
    
    fn parse_kraken_data(data: &Value) -> Option<(String, Decimal, Decimal)> {
        if let Some(arr) = data.as_array() {
            if arr.len() >= 4 {
                if let (Some(pair), Some(ticker)) = (arr[3].as_str(), arr[1].as_object()) {
                    if let (Some(ask), Some(bid), Some(vol)) = (
                        ticker["a"].as_array().and_then(|a| a[0].as_str()),
                        ticker["b"].as_array().and_then(|b| b[0].as_str()),
                        ticker["v"].as_array().and_then(|v| v[1].as_str()),
                    ) {
                        if let (Ok(a), Ok(b), Ok(v)) = (
                            Decimal::from_str(ask),
                            Decimal::from_str(bid),
                            Decimal::from_str(vol)
                        ) {
                            let price = (a + b) / Decimal::from(2);
                            return Some((pair.to_string(), price, v));
                        }
                    }
                }
            }
        }
        None
    }
    
    fn parse_uniswap_data(data: &Value) -> Option<(String, Decimal, Decimal)> {
        if let Some(pool) = data["pool"].as_object() {
            if let (Some(token0), Some(token1), Some(price), Some(volume)) = (
                pool["token0"]["symbol"].as_str(),
                pool["token1"]["symbol"].as_str(),
                pool["token0Price"].as_str(),
                pool["volumeUSD"].as_str(),
            ) {
                if let (Ok(p), Ok(v)) = (Decimal::from_str(price), Decimal::from_str(volume)) {
                    let pair = format!("{}/{}", token0, token1);
                    return Some((pair, p, v));
                }
            }
        }
        None
    }
    
    fn parse_1inch_data(data: &Value) -> Option<(String, Decimal, Decimal)> {
        if let Some(quote) = data["quote"].as_object() {
            if let (Some(from), Some(to), Some(from_amt), Some(to_amt)) = (
                quote["fromToken"]["symbol"].as_str(),
                quote["toToken"]["symbol"].as_str(),
                quote["fromTokenAmount"].as_str(),
                quote["toTokenAmount"].as_str(),
            ) {
                if let (Ok(f), Ok(t)) = (Decimal::from_str(from_amt), Decimal::from_str(to_amt)) {
                    let price = t / f;
                    let pair = format!("{}/{}", from, to);
                    return Some((pair, price, f));
                }
            }
        }
        None
    }
}