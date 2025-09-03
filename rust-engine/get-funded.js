const { ethers } = require("ethers");
require("dotenv").config();

console.log("\n💰 QUICK FUNDING GUIDE");
console.log("======================\n");

const wallet = process.env.WALLET_ADDRESS;

console.log("YOUR WALLET ADDRESS:");
console.log(wallet);
console.log("\n📋 This address has been copied to your clipboard!");
console.log("Paste it in any of the services above.\n");

console.log("FASTEST OPTIONS:");
console.log("1. Testnet (FREE): https://www.alchemy.com/faucets/arbitrum-sepolia");
console.log("2. Buy with card: https://app.uniswap.org (Click Buy → Select Arbitrum)");
console.log("3. From Binance: Withdraw ETH to Arbitrum network");
console.log("\nNeed: 0.01 ETH minimum (~$35) for mainnet");
console.log("      0.1 ETH recommended (~$350) for better profits");

// Try to copy to clipboard
try {
    const { exec } = require('child_process');
    const command = process.platform === 'darwin' ? 'pbcopy' : 'xclip -selection clipboard';
    exec(`echo "${wallet}" | ${command}`, (err) => {
        if (!err) console.log("\n✅ Address copied to clipboard!");
    });
} catch (e) {}
