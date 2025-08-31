const { ethers } = require("hardhat");
const fs = require("fs");
require("dotenv").config();

// Token addresses (Mainnet)
const TOKENS = {
  WETH: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
  USDC: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
  USDT: "0xdAC17F958D2ee523a2206206994597C13D831ec7",
  DAI: "0x6B175474E89094C44Da98b954EedeAC495271d0F"
};

// DEX Routers
const ROUTERS = {
  UNISWAP: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D",
  SUSHISWAP: "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F"
};

const ERC20_ABI = [
  "function balanceOf(address owner) view returns (uint256)",
  "function decimals() view returns (uint8)",
  "function symbol() view returns (string)",
  "function transfer(address to, uint256 amount) returns (bool)",
  "function approve(address spender, uint256 amount) returns (bool)"
];

const ROUTER_ABI = [
  "function getAmountsOut(uint amountIn, address[] memory path) view returns (uint[] memory amounts)"
];

async function main() {
  console.log("🧪 Testing Flash Loan Arbitrage on Local Fork\n");
  
  const [deployer] = await ethers.getSigners();
  
  // Use the contract address from .env or deployment.json
  const contractAddress = process.env.CONTRACT_ADDRESS || "0xbd605Ad2010E12c16B0cd0F2B8FE3c6d90BB51E7";
  console.log("✅ Using deployed contract:", contractAddress);
  
  console.log("\n📊 Checking DEX Prices...\n");
  
  const uniRouter = new ethers.Contract(ROUTERS.UNISWAP, ROUTER_ABI, deployer);
  const sushiRouter = new ethers.Contract(ROUTERS.SUSHISWAP, ROUTER_ABI, deployer);
  
  // Check USDC -> WETH prices
  const amountIn = ethers.utils.parseUnits("10000", 6); // 10,000 USDC
  
  try {
    const uniPath = [TOKENS.USDC, TOKENS.WETH];
    const uniAmounts = await uniRouter.getAmountsOut(amountIn, uniPath);
    const sushiAmounts = await sushiRouter.getAmountsOut(amountIn, uniPath);
    
    console.log("Uniswap: 10,000 USDC →", ethers.utils.formatEther(uniAmounts[1]), "WETH");
    console.log("Sushiswap: 10,000 USDC →", ethers.utils.formatEther(sushiAmounts[1]), "WETH");
    
    // Calculate price difference
    const priceDiff = uniAmounts[1].sub(sushiAmounts[1]).abs();
    const avgPrice = uniAmounts[1].add(sushiAmounts[1]).div(2);
    const diffPercent = priceDiff.mul(10000).div(avgPrice).toNumber() / 100;
    
    console.log(`\nPrice difference: ${diffPercent.toFixed(3)}%`);
    
    if (diffPercent > 0.1) {
      console.log("🎯 Potential arbitrage opportunity!");
    } else {
      console.log("❌ No significant arbitrage opportunity at current prices");
    }
    
    // Check reverse prices
    console.log("\n📊 Checking Reverse Path (WETH -> USDC)...\n");
    
    const wethAmount = ethers.utils.parseEther("1"); // 1 WETH
    const reversePath = [TOKENS.WETH, TOKENS.USDC];
    
    const uniReverse = await uniRouter.getAmountsOut(wethAmount, reversePath);
    const sushiReverse = await sushiRouter.getAmountsOut(wethAmount, reversePath);
    
    console.log("Uniswap: 1 WETH →", ethers.utils.formatUnits(uniReverse[1], 6), "USDC");
    console.log("Sushiswap: 1 WETH →", ethers.utils.formatUnits(sushiReverse[1], 6), "USDC");
    
    const reverseDiff = uniReverse[1].sub(sushiReverse[1]).abs();
    const avgReverse = uniReverse[1].add(sushiReverse[1]).div(2);
    const reverseDiffPercent = reverseDiff.mul(10000).div(avgReverse).toNumber() / 100;
    
    console.log(`\nReverse price difference: ${reverseDiffPercent.toFixed(3)}%`);
    
    // Summary
    console.log("\n" + "=".repeat(50));
    console.log("📊 SUMMARY");
    console.log("=".repeat(50));
    console.log("Contract Address:", contractAddress);
    console.log("USDC->WETH spread:", diffPercent.toFixed(3) + "%");
    console.log("WETH->USDC spread:", reverseDiffPercent.toFixed(3) + "%");
    
    if (diffPercent > 0.3 || reverseDiffPercent > 0.3) {
      console.log("\n✅ Arbitrage likely profitable (spread > 0.3%)");
      console.log("Flash loan fee: 0.09%");
      console.log("Estimated gas: ~$50-100");
    } else {
      console.log("\n⚠️ Spreads too low for profitable arbitrage");
      console.log("Need > 0.3% spread to cover flash loan fees and gas");
    }
    
  } catch (error) {
    console.error("Error:", error.message);
  }
  
  console.log("\n✅ Test Complete!");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });