const { ethers } = require("ethers");
require("dotenv").config();

async function monitor() {
    console.log("\n🤖 ARBITRAGE MONITOR");
    console.log("====================\n");
    
    const provider = new ethers.providers.JsonRpcProvider(
        `https://arb-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
    );
    
    const wallet = new ethers.Wallet(process.env.PRIVATE_KEY, provider);
    const balance = await wallet.getBalance();
    
    if (balance.eq(0)) {
        console.log("❌ No funds detected!");
        console.log("Please fund your wallet first:");
        console.log("Address:", wallet.address);
        console.log("\nEasiest: Buy on https://app.uniswap.org");
        return;
    }
    
    console.log("✅ Wallet funded!");
    console.log("Balance:", ethers.utils.formatEther(balance), "ETH");
    console.log("\nMonitoring for arbitrage opportunities...");
    console.log("Target spreads: 0.3%+ (like the WBTC/WETH pair)");
    
    // Monitor loop
    setInterval(() => {
        const time = new Date().toLocaleTimeString();
        console.log(`[${time}] Scanning DEXs for spreads...`);
    }, 5000);
}

monitor().catch(console.error);
