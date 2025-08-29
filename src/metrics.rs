use warp::Filter;
use prometheus::{Encoder, TextEncoder, Counter, Gauge, Histogram, HistogramOpts, register_counter, register_gauge, register_histogram};
use lazy_static::lazy_static;

lazy_static! {
    static ref OPPORTUNITIES_FOUND: Counter = register_counter!(
        "arbitrage_opportunities_found_total",
        "Total number of arbitrage opportunities found"
    ).unwrap();
    
    static ref PROFITABLE_OPPORTUNITIES: Counter = register_counter!(
        "arbitrage_profitable_opportunities_total",
        "Total number of profitable arbitrage opportunities"
    ).unwrap();
    
    static ref TOTAL_PROFIT: Gauge = register_gauge!(
        "arbitrage_total_profit_usd",
        "Total profit in USD from all opportunities"
    ).unwrap();
    
    static ref SCAN_DURATION: Histogram = register_histogram!(
        HistogramOpts::new(
            "arbitrage_scan_duration_seconds",
            "Duration of arbitrage scans in seconds"
        )
    ).unwrap();
    
    static ref WEBSOCKET_MESSAGES: Counter = register_counter!(
        "websocket_messages_received_total",
        "Total number of WebSocket messages received"
    ).unwrap();
    
    static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "websocket_active_connections",
        "Number of active WebSocket connections"
    ).unwrap();
    
    static ref GAS_PRICE_ETH: Gauge = register_gauge!(
        "gas_price_ethereum_gwei",
        "Current gas price on Ethereum in Gwei"
    ).unwrap();
    
    static ref POOL_COUNT: Gauge = register_gauge!(
        "liquidity_pools_tracked",
        "Number of liquidity pools being tracked"
    ).unwrap();
    
    static ref ML_ACCURACY: Gauge = register_gauge!(
        "ml_prediction_accuracy_percent",
        "Machine learning model prediction accuracy"
    ).unwrap();
}

pub struct MetricsServer {
    port: u16,
}

#[allow(dead_code)]impl MetricsServer {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
    
    pub async fn run(&self) {
        let metrics_route = warp::path("metrics")
            .map(|| {
                let encoder = TextEncoder::new();
                let metric_families = prometheus::gather();
                let mut buffer = vec![];
                encoder.encode(&metric_families, &mut buffer).unwrap();
                String::from_utf8(buffer).unwrap()
            });
        
        let health_route = warp::path("health")
            .map(|| "OK");
        
        let routes = metrics_route.or(health_route);
        
        warp::serve(routes)
            .run(([0, 0, 0, 0], self.port))
            .await;
    }
    
    pub fn record_opportunity_found() {
        OPPORTUNITIES_FOUND.inc();
    }
    
    pub fn record_profitable_opportunity() {
        PROFITABLE_OPPORTUNITIES.inc();
    }
    
    pub fn update_total_profit(profit: f64) {
        TOTAL_PROFIT.add(profit);
    }
    
    pub fn record_scan_duration(duration: f64) {
        SCAN_DURATION.observe(duration);
    }
    
    pub fn record_websocket_message() {
        WEBSOCKET_MESSAGES.inc();
    }
    
    pub fn set_active_connections(count: f64) {
        ACTIVE_CONNECTIONS.set(count);
    }
    
    pub fn set_gas_price_eth(price: f64) {
        GAS_PRICE_ETH.set(price);
    }
    
    pub fn set_pool_count(count: f64) {
        POOL_COUNT.set(count);
    }
    
    pub fn set_ml_accuracy(accuracy: f64) {
        ML_ACCURACY.set(accuracy);
    }
}