const { ethers } = require("ethers");
require("dotenv").config();

async function checkBalance() {
    const wallet = process.env.WALLET_ADDRESS;
    console.log("Wallet:", wallet);
    console.log("");
    
    // Check Arbitrum
    try {
        const provider = new ethers.providers.JsonRpcProvider(
            `https://arb-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
        );
        const balance = await provider.getBalance(wallet);
        const eth = ethers.utils.formatEther(balance);
        console.log("Arbitrum Balance:", eth, "ETH");
        
        if (parseFloat(eth) === 0) {
            console.log("Status: Need funds! See options below ⬇️");
        } else {
            console.log("Status: Ready to trade! ✅");
        }
    } catch (e) {
        console.log("Error:", e.message);
    }
}

checkBalance();
