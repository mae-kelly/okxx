use anyhow::Result;
use serde::{Deserialize, Serialize};
use reqwest::Client;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use crate::types::{ArbitrageOpportunity, Chain};
use tracing::{info, warn, error};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct DiscordNotifier {
    webhook_url: String,
    client: Client,
    min_profit_threshold: f64,
    last_notification: Arc<RwLock<DateTime<Utc>>>,
    rate_limit_seconds: i64,
}

#[derive(Debug, Serialize)]
struct DiscordWebhookMessage {
    content: Option<String>,
    embeds: Vec<DiscordEmbed>,
}

#[derive(Debug, Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<DiscordField>,
    footer: Option<DiscordFooter>,
    timestamp: String,
}

#[derive(Debug, Serialize)]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

#[derive(Debug, Serialize)]
struct DiscordFooter {
    text: String,
}

impl DiscordNotifier {
    pub fn new(webhook_url: String, min_profit_threshold: f64) -> Self {
        Self {
            webhook_url,
            client: Client::new(),
            min_profit_threshold,
            last_notification: Arc::new(RwLock::new(Utc::now() - chrono::Duration::hours(1))),
            rate_limit_seconds: 5, // Prevent spam - min 5 seconds between notifications
        }
    }
    
    pub fn from_env() -> Result<Self> {
        let webhook_url = std::env::var("DISCORD_WEBHOOK_URL")?;
        let min_profit = std::env::var("DISCORD_ALERT_MIN_PROFIT")
            .unwrap_or_else(|_| "50".to_string())
            .parse::<f64>()?;
        
        Ok(Self::new(webhook_url, min_profit))
    }
    
    pub async fn notify_opportunity(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        // Check profit threshold
        if opportunity.profit_usd < self.min_profit_threshold {
            return Ok(());
        }
        
        // Check rate limit
        {
            let last = self.last_notification.read().await;
            if (Utc::now() - *last).num_seconds() < self.rate_limit_seconds {
                return Ok(());
            }
        }
        
        let embed = self.create_opportunity_embed(opportunity);
        
        let message = DiscordWebhookMessage {
            content: Some(format!("🎯 **New Arbitrage Opportunity Found!**")),
            embeds: vec![embed],
        };
        
        match self.send_webhook(message).await {
            Ok(_) => {
                info!("Discord notification sent for opportunity {}", opportunity.id);
                let mut last = self.last_notification.write().await;
                *last = Utc::now();
                Ok(())
            }
            Err(e) => {
                warn!("Failed to send Discord notification: {}", e);
                Err(e)
            }
        }
    }
    
    pub async fn notify_high_value(&self, opportunity: &ArbitrageOpportunity) -> Result<()> {
        // For extremely high value opportunities, bypass normal rate limiting
        if opportunity.profit_usd < 500.0 {
            return self.notify_opportunity(opportunity).await;
        }
        
        let embed = self.create_high_value_embed(opportunity);
        
        let message = DiscordWebhookMessage {
            content: Some(format!("🚨 **HIGH VALUE ALERT - ${:.2} PROFIT!** 🚨", opportunity.profit_usd)),
            embeds: vec![embed],
        };
        
        self.send_webhook(message).await
    }
    
    pub async fn notify_system_status(&self, status: SystemStatus) -> Result<()> {
        let embed = self.create_status_embed(status);
        
        let message = DiscordWebhookMessage {
            content: None,
            embeds: vec![embed],
        };
        
        self.send_webhook(message).await
    }
    
    pub async fn notify_error(&self, error_msg: &str) -> Result<()> {
        let embed = DiscordEmbed {
            title: "⚠️ System Error".to_string(),
            description: error_msg.to_string(),
            color: 0xFF0000, // Red
            fields: vec![],
            footer: Some(DiscordFooter {
                text: "Arbitrage Scanner".to_string(),
            }),
            timestamp: Utc::now().to_rfc3339(),
        };
        
        let message = DiscordWebhookMessage {
            content: None,
            embeds: vec![embed],
        };
        
        self.send_webhook(message).await
    }
    
    fn create_opportunity_embed(&self, opp: &ArbitrageOpportunity) -> DiscordEmbed {
        let path_description = opp.path.iter()
            .map(|leg| format!("{} ({} → {})", leg.exchange, leg.token_in, leg.token_out))
            .collect::<Vec<_>>()
            .join(" → ");
        
        let color = if opp.profit_usd > 200.0 {
            0x00FF00 // Green for high profit
        } else if opp.profit_usd > 100.0 {
            0xFFFF00 // Yellow for medium profit
        } else {
            0x0099FF // Blue for standard
        };
        
        DiscordEmbed {
            title: format!("💰 Arbitrage: ${:.2} Profit", opp.profit_usd),
            description: format!("**Path:** {}", path_description),
            color,
            fields: vec![
                DiscordField {
                    name: "📊 ROI".to_string(),
                    value: format!("{:.2}%", opp.roi_percentage),
                    inline: true,
                },
                DiscordField {
                    name: "⛓️ Chain".to_string(),
                    value: format!("{:?}", opp.chain),
                    inline: true,
                },
                DiscordField {
                    name: "⛽ Gas Cost".to_string(),
                    value: format!("${:.2}", opp.total_gas_cost),
                    inline: true,
                },
                DiscordField {
                    name: "💸 Initial Amount".to_string(),
                    value: format!("${:.2}", opp.initial_amount),
                    inline: true,
                },
                DiscordField {
                    name: "💰 Final Amount".to_string(),
                    value: format!("${:.2}", opp.final_amount),
                    inline: true,
                },
                DiscordField {
                    name: "⚡ Flash Loan Fee".to_string(),
                    value: format!("${:.2}", opp.flash_loan_fee),
                    inline: true,
                },
                DiscordField {
                    name: "🔗 Transaction".to_string(),
                    value: format!("[View on Etherscan](https://etherscan.io/tx/{})", opp.id),
                    inline: false,
                },
            ],
            footer: Some(DiscordFooter {
                text: format!("Opportunity ID: {}", &opp.id[..8]),
            }),
            timestamp: opp.timestamp.to_rfc3339(),
        }
    }
    
    fn create_high_value_embed(&self, opp: &ArbitrageOpportunity) -> DiscordEmbed {
        let mut embed = self.create_opportunity_embed(opp);
        embed.title = format!("🚨 HIGH VALUE: ${:.2} PROFIT!", opp.profit_usd);
        embed.color = 0xFF00FF; // Magenta for urgent
        embed
    }
    
    fn create_status_embed(&self, status: SystemStatus) -> DiscordEmbed {
        DiscordEmbed {
            title: "📊 System Status Update".to_string(),
            description: "Current scanner performance metrics".to_string(),
            color: 0x00FF00,
            fields: vec![
                DiscordField {
                    name: "🔍 Opportunities Found".to_string(),
                    value: status.opportunities_found.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "💰 Total Profit".to_string(),
                    value: format!("${:.2}", status.total_profit),
                    inline: true,
                },
                DiscordField {
                    name: "⏱️ Uptime".to_string(),
                    value: format!("{} hours", status.uptime_hours),
                    inline: true,
                },
                DiscordField {
                    name: "📈 Active Chains".to_string(),
                    value: status.active_chains.join(", "),
                    inline: false,
                },
                DiscordField {
                    name: "💹 Price Feeds".to_string(),
                    value: status.price_feeds.to_string(),
                    inline: true,
                },
                DiscordField {
                    name: "🏊 Liquidity Pools".to_string(),
                    value: status.liquidity_pools.to_string(),
                    inline: true,
                },
            ],
            footer: Some(DiscordFooter {
                text: "Arbitrage Scanner v2.0".to_string(),
            }),
            timestamp: Utc::now().to_rfc3339(),
        }
    }
    
    async fn send_webhook(&self, message: DiscordWebhookMessage) -> Result<()> {
        let response = self.client
            .post(&self.webhook_url)
            .json(&message)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await?;
            error!("Discord webhook failed: {} - {}", status, text);
            return Err(anyhow::anyhow!("Discord webhook failed: {}", status));
        }
        
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SystemStatus {
    pub opportunities_found: u64,
    pub total_profit: f64,
    pub uptime_hours: u64,
    pub active_chains: Vec<String>,
    pub price_feeds: usize,
    pub liquidity_pools: usize,
}

// Integration with main scanner
pub struct NotificationManager {
    discord: Option<DiscordNotifier>,
    start_time: DateTime<Utc>,
}

impl NotificationManager {
    pub fn new() -> Result<Self> {
        let discord = DiscordNotifier::from_env().ok();
        
        if discord.is_some() {
            info!("Discord notifications enabled");
        } else {
            warn!("Discord notifications disabled - webhook not configured");
        }
        
        Ok(Self {
            discord,
            start_time: Utc::now(),
        })
    }
    
    pub async fn process_opportunity(&self, opportunity: &ArbitrageOpportunity) {
        if let Some(discord) = &self.discord {
            if opportunity.profit_usd >= 500.0 {
                let _ = discord.notify_high_value(opportunity).await;
            } else {
                let _ = discord.notify_opportunity(opportunity).await;
            }
        }
    }
    
    pub async fn send_hourly_status(&self, 
        opportunities: u64, 
        profit: f64, 
        chains: Vec<String>,
        prices: usize,
        pools: usize,
    ) {
        if let Some(discord) = &self.discord {
            let uptime = (Utc::now() - self.start_time).num_hours() as u64;
            
            let status = SystemStatus {
                opportunities_found: opportunities,
                total_profit: profit,
                uptime_hours: uptime,
                active_chains: chains,
                price_feeds: prices,
                liquidity_pools: pools,
            };
            
            let _ = discord.notify_system_status(status).await;
        }
    }
    
    pub async fn send_error(&self, error: &str) {
        if let Some(discord) = &self.discord {
            let _ = discord.notify_error(error).await;
        }
    }
}