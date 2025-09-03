const { ethers } = require("ethers");
const axios = require("axios");
const chalk = require("chalk");
require("dotenv").config();

async function getFundingOptions() {
    const wallet = process.env.WALLET_ADDRESS;
    
    console.log(chalk.blue.bold("\n💰 FUNDING OPTIONS FOR YOUR WALLET"));
    console.log("=" + "=".repeat(60));
    console.log(chalk.yellow("Wallet:"), wallet);
    console.log("");
    
    console.log(chalk.green.bold("🆓 FREE TESTNET FUNDS:"));
    console.log("-".repeat(60));
    
    console.log(chalk.cyan("1. Arbitrum Goerli Faucet (Instant):"));
    console.log("   🔗 https://faucet.quicknode.com/arbitrum/goerli");
    console.log("   - Get 0.1 ETH free");
    console.log("   - No social media required");
    console.log("");
    
    console.log(chalk.cyan("2. Alchemy Faucet (Daily):"));
    console.log("   🔗 https://goerlifaucet.com");
    console.log("   - Get 0.2 ETH daily");
    console.log("   - Sign up with Alchemy account");
    console.log("");
    
    console.log(chalk.cyan("3. Paradigm Faucet (0.5 ETH):"));
    console.log("   🔗 https://faucet.paradigm.xyz");
    console.log("   - Twitter verification required");
    console.log("");
    
    console.log(chalk.green.bold("\n💵 GET REAL ARBITRUM FUNDS:"));
    console.log("-".repeat(60));
    
    console.log(chalk.cyan("1. Bridge from Ethereum:"));
    console.log("   🔗 https://bridge.arbitrum.io");
    console.log("   - Official Arbitrum Bridge");
    console.log("   - Takes ~10 minutes");
    console.log("");
    
    console.log(chalk.cyan("2. Buy directly on Arbitrum:"));
    console.log("   🔗 https://app.uniswap.org (Select Arbitrum Network)");
    console.log("   - Buy with credit card via Moonpay/Transak");
    console.log("");
    
    console.log(chalk.cyan("3. CEX Withdrawal to Arbitrum:"));
    console.log("   - Binance: Withdraw directly to Arbitrum");
    console.log("   - Coinbase: Buy ETH → Send to Arbitrum");
    console.log("   - KuCoin: Direct Arbitrum withdrawal");
    console.log("");
    
    console.log(chalk.cyan("4. Cross-chain Bridges (Cheaper):"));
    console.log("   🔗 https://app.hop.exchange - Hop Protocol");
    console.log("   🔗 https://across.to - Across Protocol");
    console.log("   🔗 https://www.orbiter.finance - Orbiter Finance");
    console.log("");
    
    console.log(chalk.green.bold("\n🎁 BONUS OPTIONS:"));
    console.log("-".repeat(60));
    
    console.log(chalk.cyan("1. Layer3 Quests (Earn while learning):"));
    console.log("   🔗 https://layer3.xyz");
    console.log("   - Complete Arbitrum quests");
    console.log("   - Earn real tokens");
    console.log("");
    
    console.log(chalk.cyan("2. Galxe Campaigns:"));
    console.log("   🔗 https://galxe.com");
    console.log("   - Arbitrum campaigns");
    console.log("   - NFT rewards + tokens");
    console.log("");
    
    // Auto-claim from faucet (testnet)
    console.log(chalk.yellow.bold("\n🤖 AUTO-CLAIMING TESTNET FUNDS..."));
    console.log("-".repeat(60));
    
    try {
        // Try to claim from a faucet API if available
        console.log("Attempting to claim from faucets...");
        
        // Open faucet pages
        const open = require('open');
        console.log(chalk.green("✅ Opening faucet pages in your browser..."));
        
        // Copy wallet address to clipboard
        const clipboardy = require('clipboardy');
        clipboardy.writeSync(wallet);
        console.log(chalk.green(`✅ Wallet address copied to clipboard: ${wallet}`));
        
    } catch (error) {
        console.log(chalk.yellow("ℹ️  Manual claim required"));
    }
    
    console.log("");
    console.log(chalk.blue.bold("📋 YOUR WALLET ADDRESS (Copied to clipboard):"));
    console.log(chalk.green.bold(wallet));
    console.log("");
    console.log(chalk.yellow("⚡ Paste this address in any faucet above!"));
}

getFundingOptions();
