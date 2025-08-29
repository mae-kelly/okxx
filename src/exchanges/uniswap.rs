use crate::{config::ChainConfig, types::*};
use super::Exchange;
use anyhow::Result;
use async_trait::async_trait;
use ethers::{
    prelude::*,
    providers::{Provider, Http, Ws},
    types::{Address, U256},
};
use rust_decimal::Decimal;
use chrono::Utc;
use std::sync::Arc;
use std::str::FromStr;

abigen!(
    UniswapV3Factory,
    r#"[
        function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)
        function allPools(uint256) external view returns (address)
        function allPoolsLength() external view returns (uint256)
    ]"#
);

abigen!(
    UniswapV3Pool,
    r#"[
        function slot0() external view returns (uint160 sqrtPriceX96, int24 tick, uint16 observationIndex, uint16 observationCardinality, uint16 observationCardinalityNext, uint8 feeProtocol, bool unlocked)
        function liquidity() external view returns (uint128)
        function token0() external view returns (address)
        function token1() external view returns (address)
        function fee() external view returns (uint24)
    ]"#
);

abigen!(
    ERC20,
    r#"[
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
        function balanceOf(address) external view returns (uint256)
    ]"#
);

pub struct UniswapV3Exchange {
    provider: Arc<Provider<Http>>,
    chain_config: ChainConfig,
    router_address: Address,
    factory_address: Address,
    factory_contract: UniswapV3Factory<Provider<Http>>,
}

impl UniswapV3Exchange {
    pub async fn new(
        chain_config: ChainConfig,
        router_address: String,
        factory_address: String,
    ) -> Result<Self> {
        let provider = Provider::<Http>::try_from(&chain_config.rpc_urls[0])?;
        let provider = Arc::new(provider);
        
        let factory_addr = Address::from_str(&factory_address)?;
        let factory_contract = UniswapV3Factory::new(factory_addr, provider.clone());

        Ok(Self {
            provider,
            chain_config,
            router_address: Address::from_str(&router_address)?,
            factory_address: factory_addr,
            factory_contract,
        })
    }

    async fn get_pool_price(&self, pool_address: Address) -> Result<(Decimal, Decimal)> {
        let pool = UniswapV3Pool::new(pool_address, self.provider.clone());
        
        let slot0 = pool.slot_0().call().await?;
        let sqrt_price_x96 = slot0.0;
        
        let price = self.sqrt_price_to_price(sqrt_price_x96);
        let inv_price = Decimal::ONE / price;
        
        Ok((price, inv_price))
    }

    fn sqrt_price_to_price(&self, sqrt_price_x96: U256) -> Decimal {
        let q96 = U256::from(2).pow(U256::from(96));
        let q192 = q96 * q96;
        
        let price_u256 = sqrt_price_x96 * sqrt_price_x96 * U256::from(10).pow(U256::from(18)) / q192;
        
        Decimal::from_str(&price_u256.to_string()).unwrap_or(Decimal::ZERO)
    }

    async fn get_token_info(&self, token_address: Address) -> Result<Token> {
        let token = ERC20::new(token_address, self.provider.clone());
        
        let symbol = token.symbol().call().await?;
        let decimals = token.decimals().call().await?;
        
        Ok(Token {
            address: format!("{:?}", token_address),
            symbol,
            decimals,
            chain_id: self.chain_config.chain_id,
        })
    }
}

#[async_trait]
impl Exchange for UniswapV3Exchange {
    async fn get_name(&self) -> String {
        format!("UniswapV3-{}", self.chain_config.name)
    }

    async fn get_pairs(&self) -> Result<Vec<TokenPair>> {
        let mut pairs = Vec::new();
        let pool_count = self.factory_contract.all_pools_length().call().await?;
        
        let max_pools = std::cmp::min(pool_count.as_u64(), 100);
        
        for i in 0..max_pools {
            match self.factory_contract.all_pools(U256::from(i)).call().await {
                Ok(pool_address) => {
                    let pool = UniswapV3Pool::new(pool_address, self.provider.clone());
                    
                    if let (Ok(token0), Ok(token1)) = (
                        pool.token_0().call().await,
                        pool.token_1().call().await,
                    ) {
                        if let (Ok(base), Ok(quote)) = (
                            self.get_token_info(token0).await,
                            self.get_token_info(token1).await,
                        ) {
                            pairs.push(TokenPair { base, quote });
                        }
                    }
                },
                Err(_) => continue,
            }
        }
        
        Ok(pairs)
    }

    async fn get_price(&self, pair: &TokenPair) -> Result<Price> {
        let token0 = Address::from_str(&pair.base.address)?;
        let token1 = Address::from_str(&pair.quote.address)?;
        
        let fees = vec![500u32, 3000, 10000];
        let mut best_price = None;
        let mut best_liquidity = U256::zero();
        
        for fee in fees {
            if let Ok(pool_address) = self.factory_contract
                .get_pool(token0, token1, fee as u32)
                .call()
                .await
            {
                if pool_address != Address::zero() {
                    let pool = UniswapV3Pool::new(pool_address, self.provider.clone());
                    
                    if let Ok(liquidity) = pool.liquidity().call().await {
                        if liquidity > best_liquidity.as_u128() {
                            if let Ok((price, inv_price)) = self.get_pool_price(pool_address).await {
                                best_liquidity = U256::from(liquidity);
                                best_price = Some((price, inv_price));
                            }
                        }
                    }
                }
            }
        }
        
        let (price, inv_price) = best_price.ok_or_else(|| anyhow::anyhow!("No pool found"))?;
        
        Ok(Price {
            bid: price * Decimal::from_str("0.997")?,
            ask: price * Decimal::from_str("1.003")?,
            bid_size: Decimal::from(100),
            ask_size: Decimal::from(100),
            timestamp: Utc::now(),
            exchange: self.get_name().await,
            pair: pair.clone(),
        })
    }

    async fn get_orderbook(&self, pair: &TokenPair, _depth: usize) -> Result<OrderBook> {
        let price = self.get_price(pair).await?;
        
        let mut bids = Vec::new();
        let mut asks = Vec::new();
        
        for i in 0..5 {
            let price_adj = Decimal::from(1) - Decimal::from_str("0.001")? * Decimal::from(i);
            bids.push(Order {
                price: price.bid * price_adj,
                quantity: Decimal::from(100),
                timestamp: Utc::now(),
            });
            
            let price_adj = Decimal::from(1) + Decimal::from_str("0.001")? * Decimal::from(i);
            asks.push(Order {
                price: price.ask * price_adj,
                quantity: Decimal::from(100),
                timestamp: Utc::now(),
            });
        }
        
        Ok(OrderBook {
            exchange: self.get_name().await,
            pair: pair.clone(),
            bids,
            asks,
            timestamp: Utc::now(),
        })
    }

    async fn get_fees(&self) -> Result<ExchangeFees> {
        Ok(ExchangeFees {
            maker_fee: Decimal::from_str("0.003")?,
            taker_fee: Decimal::from_str("0.003")?,
            withdrawal_fee: Default::default(),
        })
    }

    async fn get_24h_volume(&self, _pair: &TokenPair) -> Result<Decimal> {
        Ok(Decimal::from(1000000))
    }

    async fn subscribe_to_updates(&self, _pairs: Vec<TokenPair>) -> Result<()> {
        Ok(())
    }
}