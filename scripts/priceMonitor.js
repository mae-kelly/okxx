const { ethers } = require("ethers");
const axios = require("axios");
require("dotenv").config();

const provider = new ethers.providers.AlchemyProvider("mainnet", process.env.ALCHEMY_API_KEY);

// Token addresses
const TOKENS = {
  WETH: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
  USDC: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  USDT: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
  DAI: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
  WBTC: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"
};

// DEX Factory addresses
const FACTORIES = {
  "Uniswap V2": "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
  "Sushiswap": "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"
};

const PAIR_ABI = [
  "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
  "function token0() external view returns (address)",
  "function token1() external view returns (address)"
];

const FACTORY_ABI = [
  "function getPair(address tokenA, address tokenB) external view returns (address pair)"
];

async function monitorPrices() {
  console.log("\n📊 Real-time Price Monitor");
  console.log("=".repeat(60));
  
  const pairs = [
    ["WETH", "USDC"],
    ["WETH", "USDT"],
    ["WETH", "DAI"],
    ["WBTC", "WETH"]
  ];
  
  while (true) {
    console.log(`\n[${new Date().toISOString()}]`);
    
    for (const [token0Name, token1Name] of pairs) {
      console.log(`\n${token0Name}/${token1Name}:`);
      
      const prices = {};
      
      for (const [dexName, factoryAddress] of Object.entries(FACTORIES)) {
        try {
          const factory = new ethers.Contract(factoryAddress, FACTORY_ABI, provider);
          const pairAddress = await factory.getPair(TOKENS[token0Name], TOKENS[token1Name]);
          
          if (pairAddress !== ethers.constants.AddressZero) {
            const pair = new ethers.Contract(pairAddress, PAIR_ABI, provider);
            const [reserve0, reserve1] = await pair.getReserves();
            const token0Address = await pair.token0();
            
            let price;
            if (token0Address.toLowerCase() === TOKENS[token0Name].toLowerCase()) {
              price = parseFloat(ethers.utils.formatUnits(reserve1, 6)) / 
                     parseFloat(ethers.utils.formatEther(reserve0));
            } else {
              price = parseFloat(ethers.utils.formatEther(reserve1)) / 
                     parseFloat(ethers.utils.formatUnits(reserve0, 6));
            }
            
            prices[dexName] = price;
            console.log(`  ${dexName}: $${price.toFixed(2)}`);
          }
        } catch (error) {
          console.log(`  ${dexName}: Error`);
        }
      }
      
      // Calculate arbitrage opportunity
      const priceValues = Object.values(prices);
      if (priceValues.length >= 2) {
        const maxPrice = Math.max(...priceValues);
        const minPrice = Math.min(...priceValues);
        const spread = ((maxPrice - minPrice) / minPrice * 100).toFixed(2);
        
        if (spread > 0.5) {
          console.log(`  🎯 Arbitrage opportunity: ${spread}% spread`);
        }
      }
    }
    
    // Check gas price
    const gasPrice = await provider.getGasPrice();
    console.log(`\n⛽ Gas: ${ethers.utils.formatUnits(gasPrice, "gwei")} gwei`);
    
    // Wait 5 seconds before next update
    await new Promise(resolve => setTimeout(resolve, 5000));
  }
}

monitorPrices().catch(console.error);