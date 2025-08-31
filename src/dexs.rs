use crate::chains::ChainManager;
use crate::types::{Chain, LiquidityPool, Token};
use anyhow::Result;
use ethers::prelude::*;
use rust_decimal::Decimal;
use std::sync::Arc;
use chrono::Utc;

// ABI for Uniswap V2 style pools
abigen!(
    IUniswapV2Pair,
    r#"[
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)
        function token0() external view returns (address)
        function token1() external view returns (address)
    ]"#
);

// ABI for ERC20 tokens
abigen!(
    IERC20,
    r#"[
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
        function balanceOf(address) external view returns (uint256)
    ]"#
);

pub struct DexManager {
    chain_manager: Arc<ChainManager>,
}

impl DexManager {
    pub async fn new(chain_manager: Arc<ChainManager>) -> Result<Self> {
        Ok(Self { chain_manager })
    }
    
    pub async fn get_pool_info(
        &self,
        chain: &Chain,
        pool_address: &str,
        dex_name: &str,
    ) -> Result<LiquidityPool> {
        let provider = self.chain_manager
            .get_provider(chain)
            .ok_or_else(|| anyhow::anyhow!("No provider for chain"))?;
        
        let pool_address = pool_address.parse::<Address>()?;
        let pool = IUniswapV2Pair::new(pool_address, provider.clone());
        
        // Get token addresses
        let token0_address = pool.token_0().call().await?;
        let token1_address = pool.token_1().call().await?;
        
        // Get reserves
        let (reserve0, reserve1, _) = pool.get_reserves().call().await?;
        
        // Get token info
        let token0 = self.get_token_info(chain, token0_address, &provider).await?;
        let token1 = self.get_token_info(chain, token1_address, &provider).await?;
        
        Ok(LiquidityPool {
            address: format!("{:?}", pool_address),
            token0,
            token1,
            reserve0: Decimal::from_str_exact(&reserve0.to_string())?,
            reserve1: Decimal::from_str_exact(&reserve1.to_string())?,
            fee: Decimal::from_str_exact("0.003")?, // 0.3% for Uniswap V2
            dex: dex_name.to_string(),
            chain: *chain,
            last_update: Utc::now(),
        })
    }
    
    async fn get_token_info(
        &self,
        chain: &Chain,
        address: Address,
        provider: &Arc<Provider<Http>>,
    ) -> Result<Token> {
        let token = IERC20::new(address, provider.clone());
        
        let symbol = token.symbol().call().await
            .unwrap_or_else(|_| "UNKNOWN".to_string());
        let decimals = token.decimals().call().await
            .unwrap_or(18);
        
        Ok(Token {
            address: format!("{:?}", address),
            symbol,
            decimals,
            chain: *chain,
        })
    }
    
    pub fn calculate_output_amount(
        &self,
        input_amount: Decimal,
        input_reserve: Decimal,
        output_reserve: Decimal,
        fee: Decimal,
    ) -> Decimal {
        let amount_with_fee = input_amount * (Decimal::ONE - fee);
        let numerator = amount_with_fee * output_reserve;
        let denominator = input_reserve + amount_with_fee;
        
        numerator / denominator
    }
    
    pub fn calculate_price_impact(
        &self,
        input_amount: Decimal,
        input_reserve: Decimal,
    ) -> Decimal {
        (input_amount / input_reserve) * Decimal::from(100)
    }
}

use rust_decimal::prelude::FromStr;