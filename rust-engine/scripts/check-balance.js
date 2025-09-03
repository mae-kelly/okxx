const { ethers } = require("ethers");
require("dotenv").config();

async function checkAllBalances() {
    const wallet = process.env.WALLET_ADDRESS;
    
    console.log("\n📊 WALLET BALANCE CHECK");
    console.log("========================");
    console.log("Wallet:", wallet);
    console.log("");
    
    // Check Arbitrum Mainnet
    try {
        const arbProvider = new ethers.providers.JsonRpcProvider(
            `https://arb-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
        );
        const arbBalance = await arbProvider.getBalance(wallet);
        const arbETH = ethers.utils.formatEther(arbBalance);
        console.log(`✅ Arbitrum Mainnet: ${arbETH} ETH`);
        
        if (parseFloat(arbETH) < 0.01) {
            console.log("   ⚠️  Need funds! See funding options below");
        }
    } catch (e) {
        console.log("❌ Arbitrum Mainnet: Connection failed");
    }
    
    // Check Arbitrum Goerli Testnet
    try {
        const testProvider = new ethers.providers.JsonRpcProvider(
            "https://goerli-rollup.arbitrum.io/rpc"
        );
        const testBalance = await testProvider.getBalance(wallet);
        const testETH = ethers.utils.formatEther(testBalance);
        console.log(`✅ Arbitrum Goerli: ${testETH} ETH (testnet)`);
    } catch (e) {
        console.log("❌ Arbitrum Goerli: Connection failed");
    }
    
    // Check Ethereum Mainnet
    try {
        const ethProvider = new ethers.providers.JsonRpcProvider(
            `https://eth-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
        );
        const ethBalance = await ethProvider.getBalance(wallet);
        const mainETH = ethers.utils.formatEther(ethBalance);
        console.log(`✅ Ethereum Mainnet: ${mainETH} ETH`);
        
        if (parseFloat(mainETH) > 0) {
            console.log("   💡 You can bridge this to Arbitrum!");
        }
    } catch (e) {
        console.log("❌ Ethereum Mainnet: Connection failed");
    }
}

checkAllBalances();
