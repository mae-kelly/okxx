use ethers::prelude::*;
use std::sync::Arc;
use anyhow::Result;
use crate::arbitrage::ArbitrageOpportunity;
use crate::config::Config;

pub struct PriceMonitor {
    provider: Arc<Provider<Ws>>,
    config: Config,
    uniswap_factory: Address,
    sushiswap_factory: Address,
}

impl PriceMonitor {
    pub fn new(provider: Arc<Provider<Ws>>, config: Config) -> Self {
        Self {
            provider,
            config: config.clone(),
            uniswap_factory: "0xf1D7CC64Fb4452F05c498126312eBE29f30Fbcf9"
                .parse::<Address>().unwrap(),
            sushiswap_factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4"
                .parse::<Address>().unwrap(),
        }
    }
    
    pub async fn find_arbitrage_opportunity(&self) -> Result<Option<ArbitrageOpportunity>> {
        let pairs = vec![
            (
                "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", // WETH
                "0xaf88d065e77c8cC2239327C5EDb3A432268e5831", // USDC
            ),
            (
                "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", // WETH
                "0x912CE59144191C1204E64559FE8253a0e49E6548", // ARB
            ),
        ];
        
        for (token_a, token_b) in pairs {
            let token_a = token_a.parse::<Address>()?;
            let token_b = token_b.parse::<Address>()?;
            
            // Get prices from both DEXs
            let (uni_price, sushi_price) = self.get_prices(token_a, token_b).await?;
            
            // Calculate price difference
            let price_diff = if uni_price > sushi_price {
                ((uni_price - sushi_price) * U256::from(10000)) / sushi_price
            } else {
                ((sushi_price - uni_price) * U256::from(10000)) / uni_price
            };
            
            // If price difference > 0.5% (50 basis points)
            if price_diff > U256::from(50) {
                let optimal_amount = self.calculate_optimal_amount(
                    token_a, 
                    token_b, 
                    uni_price, 
                    sushi_price
                ).await?;
                
                let estimated_profit = self.estimate_profit(
                    optimal_amount,
                    uni_price,
                    sushi_price
                ).await?;
                
                let gas_estimate = U256::from(400000) * self.provider.get_gas_price().await?;
                
                if estimated_profit > gas_estimate {
                    return Ok(Some(ArbitrageOpportunity {
                        token_a,
                        token_b,
                        buy_from_dex: if uni_price < sushi_price { 
                            "uniswap".to_string() 
                        } else { 
                            "sushiswap".to_string() 
                        },
                        sell_to_dex: if uni_price < sushi_price { 
                            "sushiswap".to_string() 
                        } else { 
                            "uniswap".to_string() 
                        },
                        optimal_amount,
                        estimated_profit,
                        profit_after_gas: estimated_profit - gas_estimate,
                        gas_estimate,
                    }));
                }
            }
        }
        
        Ok(None)
    }
    
    async fn get_prices(&self, token_a: Address, token_b: Address) -> Result<(U256, U256)> {
        // Get pair addresses
        let factory_abi = ethers::abi::parse_abi(&[
            "function getPair(address,address) view returns (address)"
        ])?;
        
        let pair_abi = ethers::abi::parse_abi(&[
            "function getReserves() view returns (uint112,uint112,uint32)"
        ])?;
        
        let uni_factory = Contract::new(
            self.uniswap_factory,
            factory_abi.clone(),
            self.provider.clone()
        );
        
        let sushi_factory = Contract::new(
            self.sushiswap_factory,
            factory_abi,
            self.provider.clone()
        );
        
        let uni_pair: Address = uni_factory
            .method("getPair", (token_a, token_b))?
            .call().await?;
            
        let sushi_pair: Address = sushi_factory
            .method("getPair", (token_a, token_b))?
            .call().await?;
        
        // Get reserves
        let uni_contract = Contract::new(uni_pair, pair_abi.clone(), self.provider.clone());
        let sushi_contract = Contract::new(sushi_pair, pair_abi, self.provider.clone());
        
        let uni_reserves: (U256, U256, U256) = uni_contract
            .method("getReserves", ())?
            .call().await?;
            
        let sushi_reserves: (U256, U256, U256) = sushi_contract
            .method("getReserves", ())?
            .call().await?;
        
        // Calculate prices (reserve0/reserve1)
        let uni_price = (uni_reserves.0 * U256::from(10u64.pow(18))) / uni_reserves.1;
        let sushi_price = (sushi_reserves.0 * U256::from(10u64.pow(18))) / sushi_reserves.1;
        
        Ok((uni_price, sushi_price))
    }
    
    async fn calculate_optimal_amount(
        &self,
        _token_a: Address,
        _token_b: Address,
        _uni_price: U256,
        _sushi_price: U256,
    ) -> Result<U256> {
        // Simplified optimal amount calculation
        // In production, use proper mathematical optimization
        Ok(U256::from(10u64.pow(17))) // 0.1 ETH for testing
    }
    
    async fn estimate_profit(
        &self,
        amount: U256,
        buy_price: U256,
        sell_price: U256,
    ) -> Result<U256> {
        // Simple profit calculation
        // Account for 0.3% fee on each swap
        let amount_after_buy = amount * U256::from(997) / U256::from(1000);
        let amount_after_sell = amount_after_buy * sell_price / buy_price;
        let final_amount = amount_after_sell * U256::from(997) / U256::from(1000);
        
        if final_amount > amount {
            Ok(final_amount - amount)
        } else {
            Ok(U256::zero())
        }
    }
}
