const { ethers } = require("ethers");
const fs = require("fs");
const path = require("path");
require("dotenv").config();

// Chain configurations
const CHAINS = {
  ethereum: {
    rpc: `https://eth-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`,
    chainId: 1,
    name: "Ethereum",
    dexes: {
      uniswapV2: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
      uniswapV3: "0xE592427A0AEce92De3Edee1F18E0157C05861564",
      sushiswap: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",
      balancer: "0xBA12222222228d8Ba445958a75a0704d566BF2C8",
      curve: "0x99a58482BD75cbab83b27EC03CA68fF489b5788f"
    },
    flashloan: {
      aaveV3: "0x87870Bca3F3fD6335C3F4ce8392D69350B4fA4E2",
      balancer: "0xBA12222222228d8Ba445958a75a0704d566BF2C8"
    }
  },
  arbitrum: {
    rpc: `https://arb-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`,
    chainId: 42161,
    name: "Arbitrum",
    dexes: {
      uniswapV3: "0xE592427A0AEce92De3Edee1F18E0157C05861564",
      sushiswap: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
      camelot: "0xc873fEcbd354f5A56E00E710B90EF4201db2448d"
    },
    flashloan: {
      aaveV3: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
    }
  },
  optimism: {
    rpc: `https://opt-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`,
    chainId: 10,
    name: "Optimism",
    dexes: {
      uniswapV3: "0xE592427A0AEce92De3Edee1F18E0157C05861564",
      velodrome: "0x9c12939390052919aF3155f41Bf4160Fd3666A6f"
    },
    flashloan: {
      aaveV3: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
    }
  },
  polygon: {
    rpc: `https://polygon-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`,
    chainId: 137,
    name: "Polygon",
    dexes: {
      quickswap: "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff",
      sushiswap: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506",
      uniswapV3: "0xE592427A0AEce92De3Edee1F18E0157C05861564"
    },
    flashloan: {
      aaveV3: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
    }
  }
};

// Common tokens across chains
const TOKENS = {
  ethereum: {
    WETH: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
    USDC: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    USDT: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
    DAI: "0x6B175474E89094C44Da98b954EedeAC495271d0F",
    WBTC: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599"
  },
  arbitrum: {
    WETH: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
    USDC: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
    USDT: "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
    DAI: "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1",
    WBTC: "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f"
  },
  optimism: {
    WETH: "0x4200000000000000000000000000000000000006",
    USDC: "0x7F5c764cBc14f9669B88837ca1490cCa17c31607",
    USDT: "0x94b008aA00579c1307B0EF2c499aD98a8ce58e58",
    DAI: "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1"
  },
  polygon: {
    WMATIC: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
    USDC: "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
    USDT: "0xc2132D05D31c914a87C6611C10748AEb04B58e8F",
    DAI: "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063"
  }
};

class MultiChainArbitrageScanner {
  constructor() {
    this.providers = {};
    this.opportunities = [];
    this.csvStream = null;
    this.mlProcess = null;
  }

  async initialize() {
    console.log("🚀 Initializing Multi-Chain Arbitrage Scanner\n");
    
    // Initialize providers for each chain
    for (const [chainName, config] of Object.entries(CHAINS)) {
      this.providers[chainName] = new ethers.providers.JsonRpcProvider(config.rpc);
      console.log(`✅ Connected to ${config.name}`);
    }
    
    // Initialize CSV for data collection
    this.initializeDataCollection();
    
    // Start ML process
    this.startMLProcess();
    
    console.log("\n📊 Scanner initialized. Starting continuous monitoring...\n");
  }

  initializeDataCollection() {
    const csvPath = path.join(__dirname, "../data/opportunities.csv");
    const header = "timestamp,chain,dex1,dex2,token0,token1,amount,spread,profit_usd,gas_cost,net_profit,executed,success\n";
    
    if (!fs.existsSync(csvPath)) {
      fs.writeFileSync(csvPath, header);
    }
    
    this.csvStream = fs.createWriteStream(csvPath, { flags: 'a' });
  }

  startMLProcess() {
    const { spawn } = require('child_process');
    this.mlProcess = spawn('python3', [path.join(__dirname, '../ml/arbML.py')]);
    
    this.mlProcess.stdout.on('data', (data) => {
      const recommendation = data.toString().trim();
      if (recommendation) {
        console.log(`\n🤖 ML Recommendation: ${recommendation}`);
      }
    });
  }

  async scanAllChains() {
    const allOpportunities = [];
    
    for (const [chainName, config] of Object.entries(CHAINS)) {
      const chainOpps = await this.scanChain(chainName, config);
      allOpportunities.push(...chainOpps);
    }
    
    // Sort by profitability
    allOpportunities.sort((a, b) => b.netProfit - a.netProfit);
    
    // Display top opportunities
    this.displayOpportunities(allOpportunities);
    
    // Send to ML model
    this.sendToML(allOpportunities);
    
    return allOpportunities;
  }

  async scanChain(chainName, config) {
    const opportunities = [];
    const provider = this.providers[chainName];
    const tokens = TOKENS[chainName];
    
    if (!tokens) return opportunities;
    
    // Get gas price for this chain
    const gasPrice = await provider.getGasPrice();
    const gasPriceGwei = parseFloat(ethers.utils.formatUnits(gasPrice, "gwei"));
    
    // Token pairs to check
    const pairs = this.generatePairs(tokens);
    
    for (const [token0Symbol, token1Symbol] of pairs) {
      const token0 = tokens[token0Symbol];
      const token1 = tokens[token1Symbol];
      
      if (!token0 || !token1) continue;
      
      // Check each DEX combination
      for (const [dex1Name, dex1Address] of Object.entries(config.dexes)) {
        for (const [dex2Name, dex2Address] of Object.entries(config.dexes)) {
          if (dex1Name === dex2Name) continue;
          
          try {
            const opportunity = await this.checkArbitrage(
              chainName,
              provider,
              token0,
              token1,
              token0Symbol,
              token1Symbol,
              dex1Name,
              dex1Address,
              dex2Name,
              dex2Address,
              gasPriceGwei
            );
            
            if (opportunity && opportunity.netProfit > 0) {
              opportunities.push(opportunity);
            }
          } catch (error) {
            // Silent fail - many pairs won't exist on all DEXes
          }
        }
      }
    }
    
    return opportunities;
  }

  generatePairs(tokens) {
    const pairs = [];
    const tokenSymbols = Object.keys(tokens);
    
    for (let i = 0; i < tokenSymbols.length; i++) {
      for (let j = i + 1; j < tokenSymbols.length; j++) {
        pairs.push([tokenSymbols[i], tokenSymbols[j]]);
      }
    }
    
    return pairs;
  }

  async checkArbitrage(chain, provider, token0, token1, symbol0, symbol1, dex1Name, dex1, dex2Name, dex2, gasPrice) {
    const ROUTER_ABI = [
      "function getAmountsOut(uint amountIn, address[] memory path) view returns (uint[] memory amounts)"
    ];
    
    const router1 = new ethers.Contract(dex1, ROUTER_ABI, provider);
    const router2 = new ethers.Contract(dex2, ROUTER_ABI, provider);
    
    // Test amount (in USD equivalent)
    const testAmount = ethers.utils.parseUnits("10000", 6); // 10k USDC equivalent
    
    try {
      // Get prices from both DEXes
      const path = [token0, token1];
      const amounts1 = await router1.getAmountsOut(testAmount, path);
      const amounts2 = await router2.getAmountsOut(testAmount, path);
      
      const output1 = amounts1[1];
      const output2 = amounts2[1];
      
      // Calculate spread
      const diff = output1.sub(output2).abs();
      const avg = output1.add(output2).div(2);
      const spreadBps = diff.mul(10000).div(avg).toNumber(); // Basis points
      
      if (spreadBps < 10) return null; // Less than 0.1% spread
      
      // Calculate profit
      const flashLoanFee = testAmount.mul(9).div(10000); // 0.09% Aave fee
      const estimatedGas = 500000; // Gas units
      const gasCostWei = ethers.BigNumber.from(estimatedGas).mul(gasPrice);
      
      // Estimate profit (simplified)
      const grossProfit = diff.mul(testAmount).div(avg);
      const netProfit = grossProfit.sub(flashLoanFee).sub(gasCostWei);
      
      return {
        timestamp: Date.now(),
        chain,
        dex1: dex1Name,
        dex2: dex2Name,
        token0: symbol0,
        token1: symbol1,
        amount: ethers.utils.formatUnits(testAmount, 6),
        spread: spreadBps / 100, // Convert to percentage
        grossProfit: parseFloat(ethers.utils.formatUnits(grossProfit, 6)),
        flashLoanFee: parseFloat(ethers.utils.formatUnits(flashLoanFee, 6)),
        gasCost: parseFloat(ethers.utils.formatEther(gasCostWei)) * 2000, // Assume ETH = $2000
        netProfit: parseFloat(ethers.utils.formatUnits(netProfit, 6))
      };
    } catch (error) {
      return null;
    }
  }

  displayOpportunities(opportunities) {
    console.clear();
    console.log("=" .repeat(120));
    console.log("LIVE ARBITRAGE OPPORTUNITIES FEED");
    console.log("=" .repeat(120));
    console.log(`Time: ${new Date().toLocaleTimeString()}`);
    console.log("-" .repeat(120));
    
    // Header
    console.log(
      "Chain".padEnd(12) +
      "DEX Route".padEnd(25) +
      "Pair".padEnd(15) +
      "Spread %".padEnd(10) +
      "Gross $".padEnd(10) +
      "FL Fee $".padEnd(10) +
      "Gas $".padEnd(10) +
      "Net Profit $".padEnd(12) +
      "Status"
    );
    console.log("-" .repeat(120));
    
    // Show top 20 opportunities
    const top20 = opportunities.slice(0, 20);
    
    for (const opp of top20) {
      const route = `${opp.dex1}→${opp.dex2}`;
      const pair = `${opp.token0}/${opp.token1}`;
      const status = opp.netProfit > 100 ? "🟢 HOT" : opp.netProfit > 50 ? "🟡 WARM" : "🔵 COOL";
      
      console.log(
        opp.chain.padEnd(12) +
        route.padEnd(25) +
        pair.padEnd(15) +
        `${opp.spread.toFixed(2)}%`.padEnd(10) +
        `$${opp.grossProfit.toFixed(2)}`.padEnd(10) +
        `$${opp.flashLoanFee.toFixed(2)}`.padEnd(10) +
        `$${opp.gasCost.toFixed(2)}`.padEnd(10) +
        `$${opp.netProfit.toFixed(2)}`.padEnd(12) +
        status
      );
      
      // Log to CSV
      this.logToCSV(opp);
    }
    
    console.log("-" .repeat(120));
    console.log(`Total opportunities found: ${opportunities.length}`);
    console.log(`Profitable (>$0): ${opportunities.filter(o => o.netProfit > 0).length}`);
    console.log(`High profit (>$100): ${opportunities.filter(o => o.netProfit > 100).length}`);
  }

  logToCSV(opportunity) {
    const row = `${opportunity.timestamp},${opportunity.chain},${opportunity.dex1},${opportunity.dex2},${opportunity.token0},${opportunity.token1},${opportunity.amount},${opportunity.spread},${opportunity.grossProfit},${opportunity.gasCost},${opportunity.netProfit},false,null\n`;
    this.csvStream.write(row);
  }

  sendToML(opportunities) {
    if (this.mlProcess && opportunities.length > 0) {
      const data = JSON.stringify(opportunities.slice(0, 10)); // Send top 10
      this.mlProcess.stdin.write(data + '\n');
    }
  }

  async start() {
    await this.initialize();
    
    // Continuous scanning
    setInterval(async () => {
      await this.scanAllChains();
    }, 3000); // Every 3 seconds
    
    // Initial scan
    await this.scanAllChains();
  }
}

// Run the scanner
const scanner = new MultiChainArbitrageScanner();
scanner.start().catch(console.error);