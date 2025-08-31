const { ethers } = require("hardhat");

async function main() {
  console.log("💰 Executing Arbitrage Trade\n");
  
  const [deployer] = await ethers.getSigners();
  const contractAddress = "0xbd605Ad2010E12c16B0cd0F2B8FE3c6d90BB51E7";
  
  // Get contract ABI
  const contractABI = require("../artifacts/contracts/FlashLoanArbitrage.sol/FlashLoanArbitrage.json").abi;
  const arbitrageContract = new ethers.Contract(contractAddress, contractABI, deployer);
  
  // Token addresses
  const USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
  const WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
  
  // Routers (buy on Sushiswap, sell on Uniswap based on your prices)
  const SUSHISWAP = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
  const UNISWAP = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
  
  const flashLoanAmount = ethers.utils.parseUnits("100000", 6); // 100k USDC
  const minProfit = ethers.utils.parseUnits("100", 6); // Min $100 profit
  
  console.log("Flash Loan: 100,000 USDC");
  console.log("Buy on: Sushiswap (cheaper)");
  console.log("Sell on: Uniswap (higher price)");
  console.log("Expected profit: ~$440 (0.44% of 100k)\n");
  
  try {
    const tx = await arbitrageContract.executeArbitrage(
      USDC,
      flashLoanAmount,
      SUSHISWAP,  // Buy router (cheaper)
      UNISWAP,    // Sell router (higher price)
      [USDC, WETH],  // Buy path
      [WETH, USDC],  // Sell path
      minProfit,
      { gasLimit: 800000 }
    );
    
    console.log("📤 Transaction sent:", tx.hash);
    const receipt = await tx.wait();
    console.log("✅ Transaction confirmed!");
    
    // Check events
    const event = receipt.events?.find(e => e.event === "ArbitrageExecuted");
    if (event) {
      const profit = ethers.utils.formatUnits(event.args.profit, 6);
      console.log(`\n💰 PROFIT: $${profit} USDC`);
    }
    
  } catch (error) {
    console.error("❌ Failed:", error.reason || error.message);
    
    // Common errors and solutions
    if (error.message.includes("Insufficient profit")) {
      console.log("\nPrices changed during execution - normal in volatile markets");
    } else if (error.message.includes("transfer amount exceeds balance")) {
      console.log("\nNeed to fund contract or adjust amounts");
    }
  }
}

main().catch(console.error);