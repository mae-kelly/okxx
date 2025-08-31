const { ethers } = require("ethers");
const fs = require("fs");
require("dotenv").config();

class UltimateArbitrageScanner {
  constructor() {
    this.provider = new ethers.providers.JsonRpcProvider(
      `https://eth-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
    );
    
    // DEXes to scan
    this.DEXES = {
      uniswapV2: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
      sushiswap: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F",
      shibaswap: "0x03f7724180AA6b939894B5Ca4314783B0b36b329"
    };
    
    // Flash loan providers and their fees
    this.FLASH_LOAN_PROVIDERS = {
      aaveV3: { fee: 0.0009, maxLoan: 1000000000 }, // 0.09% fee
      balancer: { fee: 0, maxLoan: 500000000 },      // 0% fee
      dydx: { fee: 0.0002, maxLoan: 100000000 }      // 0.02% fee
    };
    
    // All major tokens with accurate decimals and current prices
    this.TOKENS = {
      WETH: { address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", decimals: 18, price: 4500 },
      USDC: { address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", decimals: 6, price: 1 },
      USDT: { address: "0xdAC17F958D2ee523a2206206994597C13D831ec7", decimals: 6, price: 1 },
      DAI: { address: "0x6B175474E89094C44Da98b954EedeAC495271d0F", decimals: 18, price: 1 },
      WBTC: { address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", decimals: 8, price: 95000 },
      LINK: { address: "0x514910771AF9Ca656af840dff83E8264EcF986CA", decimals: 18, price: 15 },
      UNI: { address: "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984", decimals: 18, price: 8 },
      AAVE: { address: "0x7Fc66500c84A76Ad7e9c93437bFc5Ac33E2DDaE9", decimals: 18, price: 120 },
      MKR: { address: "0x9f8F72aA9304c8B593d555F12eF6589cC3A579A2", decimals: 18, price: 2000 }
    };
    
    this.opportunities = [];
  }
  
  async getCurrentGasPrice() {
    const gasPrice = await this.provider.getGasPrice();
    const gasPriceGwei = parseFloat(ethers.utils.formatUnits(gasPrice, "gwei"));
    const gasUnits = 350000; // Typical arbitrage transaction
    const ethPrice = this.TOKENS.WETH.price;
    const gasCostUSD = (gasPriceGwei * gasUnits * ethPrice) / 1000000000;
    return { gasPriceGwei, gasCostUSD };
  }
  
  getBestFlashLoanProvider(amount) {
    let bestProvider = null;
    let lowestFee = Infinity;
    
    for (const [name, details] of Object.entries(this.FLASH_LOAN_PROVIDERS)) {
      if (amount <= details.maxLoan) {
        const fee = amount * details.fee;
        if (fee < lowestFee) {
          lowestFee = fee;
          bestProvider = { name, fee: details.fee, totalFee: fee };
        }
      }
    }
    
    return bestProvider;
  }
  
  async scanAllPairs() {
    console.clear();
    const { gasPriceGwei, gasCostUSD } = await this.getCurrentGasPrice();
    
    console.log("=".repeat(130));
    console.log("ULTIMATE ARBITRAGE SCANNER - REAL NET PROFIT CALCULATIONS");
    console.log("=".repeat(130));
    console.log(`Time: ${new Date().toLocaleTimeString()} | Gas: ${gasPriceGwei.toFixed(1)} gwei ($${gasCostUSD.toFixed(2)})`);
    console.log("-".repeat(130));
    
    const routerABI = ["function getAmountsOut(uint,address[]) view returns(uint[])"];
    const opportunities = [];
    
    const tokenNames = Object.keys(this.TOKENS);
    
    // Scan all token pairs
    for (let i = 0; i < tokenNames.length; i++) {
      for (let j = i + 1; j < tokenNames.length; j++) {
        const token0Name = tokenNames[i];
        const token1Name = tokenNames[j];
        const token0 = this.TOKENS[token0Name];
        const token1 = this.TOKENS[token1Name];
        
        // Test with realistic trade size based on token value
        const tradeValueUSD = 100000; // $100k trade
        const token0Amount = tradeValueUSD / token0.price;
        const amountIn = ethers.utils.parseUnits(
          token0Amount.toFixed(Math.min(6, token0.decimals)), 
          token0.decimals
        );
        
        const prices = {};
        
        // Get prices from each DEX
        for (const [dexName, routerAddress] of Object.entries(this.DEXES)) {
          try {
            const router = new ethers.Contract(routerAddress, routerABI, this.provider);
            const result = await router.getAmountsOut(amountIn, [token0.address, token1.address]);
            prices[dexName] = result[1];
          } catch (e) {
            // Pair doesn't exist on this DEX
          }
        }
        
        // Find arbitrage between DEXes
        const dexList = Object.keys(prices);
        
        for (let a = 0; a < dexList.length; a++) {
          for (let b = a + 1; b < dexList.length; b++) {
            const dex1 = dexList[a];
            const dex2 = dexList[b];
            const price1 = prices[dex1];
            const price2 = prices[dex2];
            
            // Determine arbitrage direction
            let buyDex, sellDex, priceDiff;
            
            if (price1.gt(price2)) {
              buyDex = dex2;
              sellDex = dex1;
              priceDiff = price1.sub(price2);
            } else {
              buyDex = dex1;
              sellDex = dex2;
              priceDiff = price2.sub(price1);
            }
            
            // Calculate spread percentage
            const avgPrice = price1.add(price2).div(2);
            const spreadBps = priceDiff.mul(10000).div(avgPrice);
            const spreadPercent = spreadBps.toNumber() / 100;
            
            if (spreadPercent > 0.05) { // Only consider spreads > 0.05%
              // Calculate gross profit in USD
              const token1AmountDiff = parseFloat(ethers.utils.formatUnits(priceDiff, token1.decimals));
              const grossProfitUSD = token1AmountDiff * token1.price;
              
              // Get best flash loan provider
              const flashLoan = this.getBestFlashLoanProvider(tradeValueUSD);
              const flashLoanFeeUSD = flashLoan ? flashLoan.totalFee : tradeValueUSD * 0.0009;
              
              // Calculate net profit
              const netProfitUSD = grossProfitUSD - flashLoanFeeUSD - gasCostUSD;
              
              opportunities.push({
                token0: token0Name,
                token1: token1Name,
                buyDex,
                sellDex,
                tradeSize: tradeValueUSD,
                spreadPercent,
                grossProfitUSD,
                flashLoanProvider: flashLoan ? flashLoan.name : 'aaveV3',
                flashLoanFeeUSD,
                gasCostUSD,
                netProfitUSD
              });
            }
          }
        }
      }
    }
    
    // Sort by net profit
    opportunities.sort((a, b) => b.netProfitUSD - a.netProfitUSD);
    
    // Display header
    console.log(
      "Token Pair".padEnd(15) +
      "Route".padEnd(22) +
      "Trade Size".padEnd(12) +
      "Spread %".padEnd(10) +
      "Gross $".padEnd(10) +
      "FL Provider".padEnd(12) +
      "FL Fee $".padEnd(10) +
      "Gas $".padEnd(8) +
      "NET PROFIT".padEnd(12) +
      "Status"
    );
    console.log("-".repeat(130));
    
    // Display opportunities
    const displayCount = Math.min(25, opportunities.length);
    
    for (let i = 0; i < displayCount; i++) {
      const opp = opportunities[i];
      
      let status = "";
      if (opp.netProfitUSD > 500) status = "🔥🔥🔥 INSTANT";
      else if (opp.netProfitUSD > 200) status = "🔥🔥 EXECUTE";
      else if (opp.netProfitUSD > 100) status = "🔥 HOT";
      else if (opp.netProfitUSD > 50) status = "✅ GOOD";
      else if (opp.netProfitUSD > 0) status = "📊 OK";
      else status = "❌ LOSS";
      
      console.log(
        `${opp.token0}/${opp.token1}`.padEnd(15) +
        `${opp.buyDex}→${opp.sellDex}`.padEnd(22) +
        `$${(opp.tradeSize/1000).toFixed(0)}k`.padEnd(12) +
        `${opp.spreadPercent.toFixed(3)}%`.padEnd(10) +
        `$${opp.grossProfitUSD.toFixed(0)}`.padEnd(10) +
        opp.flashLoanProvider.padEnd(12) +
        `$${opp.flashLoanFeeUSD.toFixed(0)}`.padEnd(10) +
        `$${opp.gasCostUSD.toFixed(0)}`.padEnd(8) +
        `$${opp.netProfitUSD.toFixed(2)}`.padEnd(12) +
        status
      );
    }
    
    console.log("-".repeat(130));
    
    // Summary statistics
    const profitable = opportunities.filter(o => o.netProfitUSD > 0);
    const highProfit = opportunities.filter(o => o.netProfitUSD > 100);
    const instant = opportunities.filter(o => o.netProfitUSD > 500);
    
    console.log(`SUMMARY: ${opportunities.length} opportunities | ${profitable.length} profitable | ${highProfit.length} high (>$100) | ${instant.length} instant (>$500)`);
    
    if (instant.length > 0) {
      console.log("\n⚡ INSTANT EXECUTION RECOMMENDED:");
      const best = instant[0];
      console.log(`   ${best.token0}/${best.token1} on ${best.buyDex}→${best.sellDex}`);
      console.log(`   Net Profit: $${best.netProfitUSD.toFixed(2)}`);
      console.log(`   Flash Loan: ${best.flashLoanProvider} ($${best.flashLoanFeeUSD.toFixed(0)} fee)`);
    }
    
    // Save to CSV for ML
    this.saveToCSV(opportunities);
    
    return opportunities;
  }
  
  saveToCSV(opportunities) {
    if (!fs.existsSync('data')) fs.mkdirSync('data');
    
    const filename = `data/arb_${new Date().toISOString().split('T')[0]}.csv`;
    
    if (!fs.existsSync(filename)) {
      const header = "timestamp,token0,token1,buyDex,sellDex,tradeSize,spread%,gross$,flashProvider,flashFee$,gas$,netProfit$\n";
      fs.writeFileSync(filename, header);
    }
    
    opportunities.forEach(opp => {
      const row = `${Date.now()},${opp.token0},${opp.token1},${opp.buyDex},${opp.sellDex},${opp.tradeSize},${opp.spreadPercent},${opp.grossProfitUSD},${opp.flashLoanProvider},${opp.flashLoanFeeUSD},${opp.gasCostUSD},${opp.netProfitUSD}\n`;
      fs.appendFileSync(filename, row);
    });
  }
  
  async start() {
    console.log("Starting Ultimate Scanner...\n");
    
    // Initial scan
    await this.scanAllPairs();
    
    // Scan every 3 seconds
    setInterval(async () => {
      await this.scanAllPairs();
    }, 3000);
  }
}

const scanner = new UltimateArbitrageScanner();
scanner.start().catch(console.error);