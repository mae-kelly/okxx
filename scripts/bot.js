const { ethers } = require("hardhat");
const axios = require("axios");
const cron = require("node-cron");
const winston = require("winston");
require("dotenv").config();

// Logger setup
const logger = winston.createLogger({
  level: 'info',
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.json()
  ),
  transports: [
    new winston.transports.File({ filename: 'error.log', level: 'error' }),
    new winston.transports.File({ filename: 'arbitrage.log' }),
    new winston.transports.Console({
      format: winston.format.simple()
    })
  ]
});

// Configuration
const CONFIG = {
  MIN_PROFIT_USD: parseFloat(process.env.MIN_PROFIT_THRESHOLD || "100"),
  MAX_GAS_PRICE: ethers.utils.parseUnits(process.env.GAS_PRICE_LIMIT || "50", "gwei"),
  SLIPPAGE_TOLERANCE: parseInt(process.env.SLIPPAGE_TOLERANCE || "3"),
  FLASH_LOAN_PREMIUM: 0.0009, // Aave V3 premium
  CHECK_INTERVAL: "*/3 * * * * *", // Every 3 seconds
};

// Contract addresses
const CONTRACTS = {
  ARBITRAGE_CONTRACT: process.env.CONTRACT_ADDRESS,
  AAVE_POOL: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2",
  UNISWAP_V2_ROUTER: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
  SUSHISWAP_ROUTER: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",
  SHIBASWAP_ROUTER: "0x03f7724180AA6b939894B5Ca4314783B0b36b329",
  UNISWAP_V2_FACTORY: "0x5C69bEe701ef814a2B6a3EDD4B1652CB9cc5aA6f",
  SUSHISWAP_FACTORY: "0xC0AEe478e3658e2610c5F7A4A2E1777cE9e4f2Ac"
};

// Token addresses
const TOKENS = {
  WETH: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
  USDC: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  USDT: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
  DAI: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
  WBTC: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
  LINK: "0x514910771AF9Ca656af840dff83E8264EcF986CA",
  UNI: "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984",
  AAVE: "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9"
};

// ABIs
const ROUTER_ABI = [
  "function getAmountsOut(uint amountIn, address[] memory path) public view returns (uint[] memory amounts)",
  "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] calldata path, address to, uint deadline) external returns (uint[] memory amounts)"
];

const FACTORY_ABI = [
  "function getPair(address tokenA, address tokenB) external view returns (address pair)"
];

const PAIR_ABI = [
  "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
  "function token0() external view returns (address)",
  "function token1() external view returns (address)"
];

const ERC20_ABI = [
  "function decimals() external view returns (uint8)",
  "function symbol() external view returns (string)",
  "function balanceOf(address account) external view returns (uint256)"
];

class ArbitrageBot {
  constructor() {
    this.provider = new ethers.providers.AlchemyProvider("mainnet", process.env.ALCHEMY_API_KEY);
    this.wallet = new ethers.Wallet(process.env.PRIVATE_KEY, this.provider);
    this.contract = null;
    this.isExecuting = false;
    this.tokenDecimals = {};
    this.tokenPrices = {};
  }

  async initialize() {
    logger.info("Initializing Arbitrage Bot...");
    
    // Load contract
    const contractABI = require("../artifacts/contracts/FlashLoanArbitrage.sol/FlashLoanArbitrage.json").abi;
    this.contract = new ethers.Contract(CONTRACTS.ARBITRAGE_CONTRACT, contractABI, this.wallet);
    
    // Initialize token decimals
    await this.loadTokenDecimals();
    
    // Load initial prices
    await this.updateTokenPrices();
    
    logger.info(`Bot initialized - Contract: ${CONTRACTS.ARBITRAGE_CONTRACT}`);
    logger.info(`Wallet: ${this.wallet.address}`);
    
    // Send Discord notification if webhook is configured
    if (process.env.DISCORD_WEBHOOK) {
      await this.sendDiscordNotification(
        "🤖 Arbitrage Bot Started",
        `Bot is monitoring for opportunities\nWallet: ${this.wallet.address}`
      );
    }
  }

  async loadTokenDecimals() {
    for (const [symbol, address] of Object.entries(TOKENS)) {
      const token = new ethers.Contract(address, ERC20_ABI, this.provider);
      this.tokenDecimals[address] = await token.decimals();
    }
  }

  async updateTokenPrices() {
    try {
      const response = await axios.get(
        `https://api.coingecko.com/api/v3/simple/token_price/ethereum`,
        {
          params: {
            contract_addresses: Object.values(TOKENS).join(','),
            vs_currencies: 'usd'
          }
        }
      );
      
      for (const [address, data] of Object.entries(response.data)) {
        this.tokenPrices[address.toLowerCase()] = data.usd;
      }
    } catch (error) {
      logger.error("Failed to update token prices:", error.message);
    }
  }

  async findArbitrageOpportunities() {
    const opportunities = [];
    
    // Token pairs to check
    const pairs = [
      [TOKENS.WETH, TOKENS.USDC],
      [TOKENS.WETH, TOKENS.USDT],
      [TOKENS.WETH, TOKENS.DAI],
      [TOKENS.WBTC, TOKENS.WETH],
      [TOKENS.WBTC, TOKENS.USDC],
      [TOKENS.LINK, TOKENS.WETH],
      [TOKENS.UNI, TOKENS.WETH],
      [TOKENS.AAVE, TOKENS.WETH]
    ];

    const dexes = [
      { name: "Uniswap", router: CONTRACTS.UNISWAP_V2_ROUTER, factory: CONTRACTS.UNISWAP_V2_FACTORY },
      { name: "Sushiswap", router: CONTRACTS.SUSHISWAP_ROUTER, factory: CONTRACTS.SUSHISWAP_FACTORY }
    ];

    for (const [token0, token1] of pairs) {
      try {
        // Get prices from each DEX
        const prices = [];
        
        for (const dex of dexes) {
          const price = await this.getPrice(dex, token0, token1);
          if (price) {
            prices.push({ ...price, dex });
          }
        }
        
        if (prices.length >= 2) {
          // Find best arbitrage opportunity
          for (let i = 0; i < prices.length; i++) {
            for (let j = 0; j < prices.length; j++) {
              if (i !== j) {
                const buyDex = prices[i];
                const sellDex = prices[j];
                
                const opportunity = await this.calculateArbitrage(
                  token0,
                  token1,
                  buyDex,
                  sellDex
                );
                
                if (opportunity && opportunity.profitUSD > CONFIG.MIN_PROFIT_USD) {
                  opportunities.push(opportunity);
                }
              }
            }
          }
        }
      } catch (error) {
        logger.error(`Error checking pair ${token0}/${token1}:`, error.message);
      }
    }
    
    return opportunities;
  }

  async getPrice(dex, token0, token1) {
    try {
      const router = new ethers.Contract(dex.router, ROUTER_ABI, this.provider);
      const factory = new ethers.Contract(dex.factory, FACTORY_ABI, this.provider);
      
      // Get pair address
      const pairAddress = await factory.getPair(token0, token1);
      if (pairAddress === ethers.constants.AddressZero) {
        return null;
      }
      
      // Get reserves
      const pair = new ethers.Contract(pairAddress, PAIR_ABI, this.provider);
      const [reserve0, reserve1] = await pair.getReserves();
      const token0Address = await pair.token0();
      
      // Calculate price
      const decimals0 = this.tokenDecimals[token0];
      const decimals1 = this.tokenDecimals[token1];
      
      let price;
      if (token0Address.toLowerCase() === token0.toLowerCase()) {
        price = reserve1.mul(ethers.BigNumber.from(10).pow(decimals0))
          .div(reserve0.mul(ethers.BigNumber.from(10).pow(decimals1)));
      } else {
        price = reserve0.mul(ethers.BigNumber.from(10).pow(decimals1))
          .div(reserve1.mul(ethers.BigNumber.from(10).pow(decimals0)));
      }
      
      return {
        price,
        reserve0,
        reserve1,
        pairAddress
      };
    } catch (error) {
      return null;
    }
  }

  async calculateArbitrage(token0, token1, buyDex, sellDex) {
    try {
      // Calculate price difference
      const priceDiff = sellDex.price.sub(buyDex.price);
      if (priceDiff.lte(0)) return null;
      
      // Calculate optimal trade amount (simplified)
      const tradeAmount = ethers.utils.parseUnits("1000", 6); // Start with $1000 USDC
      
      // Estimate gas costs
      const gasPrice = await this.provider.getGasPrice();
      const gasLimit = ethers.BigNumber.from("500000");
      const gasCost = gasPrice.mul(gasLimit);
      const gasCostUSD = parseFloat(ethers.utils.formatEther(gasCost)) * 
        (this.tokenPrices[TOKENS.WETH.toLowerCase()] || 2000);
      
      // Calculate profit
      const buyPath = [token0, token1];
      const sellPath = [token1, token0];
      
      const buyRouter = new ethers.Contract(buyDex.dex.router, ROUTER_ABI, this.provider);
      const sellRouter = new ethers.Contract(sellDex.dex.router, ROUTER_ABI, this.provider);
      
      const buyAmounts = await buyRouter.getAmountsOut(tradeAmount, buyPath);
      const outputAmount = buyAmounts[buyAmounts.length - 1];
      
      const sellAmounts = await sellRouter.getAmountsOut(outputAmount, sellPath);
      const finalAmount = sellAmounts[sellAmounts.length - 1];
      
      // Include flash loan fee
      const flashLoanFee = tradeAmount.mul(9).div(10000); // 0.09%
      const totalCost = tradeAmount.add(flashLoanFee);
      
      if (finalAmount.lte(totalCost)) return null;
      
      const profit = finalAmount.sub(totalCost);
      const profitUSD = parseFloat(ethers.utils.formatUnits(profit, 6));
      
      // Check if profitable after gas
      if (profitUSD <= gasCostUSD) return null;
      
      return {
        token0,
        token1,
        buyDex: buyDex.dex.name,
        buyRouter: buyDex.dex.router,
        sellDex: sellDex.dex.name,
        sellRouter: sellDex.dex.router,
        tradeAmount,
        expectedProfit: profit,
        profitUSD: profitUSD - gasCostUSD,
        gasCostUSD,
        buyPath,
        sellPath
      };
    } catch (error) {
      logger.error("Error calculating arbitrage:", error.message);
      return null;
    }
  }

  async executeArbitrage(opportunity) {
    if (this.isExecuting) return;
    this.isExecuting = true;
    
    try {
      logger.info(`Executing arbitrage: ${opportunity.buyDex} -> ${opportunity.sellDex}`);
      logger.info(`Expected profit: $${opportunity.profitUSD.toFixed(2)}`);
      
      // Check gas price
      const gasPrice = await this.provider.getGasPrice();
      if (gasPrice.gt(CONFIG.MAX_GAS_PRICE)) {
        logger.warn(`Gas price too high: ${ethers.utils.formatUnits(gasPrice, "gwei")} gwei`);
        return;
      }
      
      // Build transaction
      const minProfit = ethers.utils.parseUnits(
        Math.floor(opportunity.profitUSD * 0.8).toString(), 
        6
      ); // 80% of expected profit as minimum
      
      const tx = await this.contract.executeArbitrage(
        opportunity.token0,
        opportunity.tradeAmount,
        opportunity.buyRouter,
        opportunity.sellRouter,
        opportunity.buyPath,
        opportunity.sellPath,
        minProfit,
        {
          gasLimit: 600000,
          gasPrice: gasPrice.mul(110).div(100), // 10% buffer
          type: 2,
          maxFeePerGas: gasPrice.mul(120).div(100),
          maxPriorityFeePerGas: ethers.utils.parseUnits("2", "gwei")
        }
      );
      
      logger.info(`Transaction sent: ${tx.hash}`);
      
      const receipt = await tx.wait();
      logger.info(`Transaction confirmed: ${receipt.transactionHash}`);
      logger.info(`Gas used: ${receipt.gasUsed.toString()}`);
      
      // Send Discord notification
      if (process.env.DISCORD_WEBHOOK) {
        await this.sendDiscordNotification(
          "💰 Arbitrage Executed!",
          `Profit: $${opportunity.profitUSD.toFixed(2)}\nRoute: ${opportunity.buyDex} → ${opportunity.sellDex}\nTx: ${receipt.transactionHash}`
        );
      }
      
    } catch (error) {
      logger.error("Execution failed:", error.message);
      
      if (process.env.DISCORD_WEBHOOK) {
        await this.sendDiscordNotification(
          "❌ Arbitrage Failed",
          `Error: ${error.message}\nRoute: ${opportunity.buyDex} → ${opportunity.sellDex}`
        );
      }
    } finally {
      this.isExecuting = false;
    }
  }

  async sendDiscordNotification(title, description) {
    if (!process.env.DISCORD_WEBHOOK) return;
    
    try {
      await axios.post(process.env.DISCORD_WEBHOOK, {
        embeds: [{
          title,
          description,
          color: title.includes("Failed") ? 0xff0000 : 0x00ff00,
          timestamp: new Date().toISOString(),
          footer: {
            text: "Flash Loan Arbitrage Bot"
          }
        }]
      });
    } catch (error) {
      logger.error("Discord notification failed:", error.message);
    }
  }

  async checkHealth() {
    try {
      // Check wallet balance
      const balance = await this.wallet.getBalance();
      const balanceETH = parseFloat(ethers.utils.formatEther(balance));
      
      if (balanceETH < 0.1) {
        logger.warn(`Low ETH balance: ${balanceETH} ETH`);
        if (process.env.DISCORD_WEBHOOK) {
          await this.sendDiscordNotification(
            "⚠️ Low Balance Warning",
            `Wallet ETH balance: ${balanceETH.toFixed(4)} ETH`
          );
        }
      }
      
      // Check contract authorization
      const isAuthorized = await this.contract.authorizedCallers(this.wallet.address);
      if (!isAuthorized && this.wallet.address !== await this.contract.owner()) {
        logger.error("Wallet not authorized on contract");
      }
      
      // Update token prices
      await this.updateTokenPrices();
      
    } catch (error) {
      logger.error("Health check failed:", error.message);
    }
  }

  async run() {
    if (this.isExecuting) return;
    
    try {
      // Health check every 10 runs
      if (Math.random() < 0.1) {
        await this.checkHealth();
      }
      
      // Find opportunities
      const opportunities = await this.findArbitrageOpportunities();
      
      if (opportunities.length > 0) {
        logger.info(`Found ${opportunities.length} opportunities`);
        
        // Sort by profit
        opportunities.sort((a, b) => b.profitUSD - a.profitUSD);
        
        // Execute best opportunity
        await this.executeArbitrage(opportunities[0]);
      }
      
    } catch (error) {
      logger.error("Bot error:", error.message);
    }
  }

  start() {
    logger.info("Starting arbitrage bot...");
    
    // Run immediately
    this.run();
    
    // Schedule periodic runs
    cron.schedule(CONFIG.CHECK_INTERVAL, () => {
      this.run();
    });
    
    logger.info(`Bot scheduled to run every ${CONFIG.CHECK_INTERVAL}`);
  }
}

// Main execution
async function main() {
  const bot = new ArbitrageBot();
  
  try {
    await bot.initialize();
    bot.start();
    
    // Handle shutdown
    process.on("SIGINT", async () => {
      logger.info("Shutting down bot...");
      if (process.env.DISCORD_WEBHOOK) {
        await bot.sendDiscordNotification(
          "🛑 Bot Stopped",
          "Arbitrage bot has been shut down"
        );
      }
      process.exit(0);
    });
    
  } catch (error) {
    logger.error("Fatal error:", error);
    process.exit(1);
  }
}

main();