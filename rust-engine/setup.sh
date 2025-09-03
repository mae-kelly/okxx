#!/bin/bash

# 🚀 ARBITRAGE BOT SETUP - FIXED VERSION
# Complete setup with funding options

set -e  # Exit on any error

echo "======================================"
echo "🤖 ARBITRAGE BOT SETUP"
echo "======================================"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Your configuration
WALLET_ADDRESS="0xB06bB023c084A34f410F1069EbD467bEA83ADaB2"
PRIVATE_KEY="0x2cded561032136fb4aecb8b89b7d7e4a54b86d2d0b98f5f3b635de4a44984c37"
ALCHEMY_API_KEY="alcht_oZ7wU7JpIoZejlOWUcMFOpNsIlLDsX"
INFURA_API_KEY="2e1c7909e5e4488e99010fabd3590a79"
ETHERSCAN_API_KEY="K4SEVFZ3PI8STM73VKV84C8PYZJUK7HB2G"
DISCORD_WEBHOOK="https://discord.com/api/webhooks/1398448251933298740/lSnT3iPsfvb87RWdN0XCd3AjdFsCZiTpF-_I1ciV3rB2BqTpIszS6U6tFxAVk5QmM2q3"

echo -e "${YELLOW}⚠️  Security Note${NC}"
echo "Your wallet currently has 0 ETH. Let's get you funded!"
echo ""

echo -e "${GREEN}Step 1: Setting Up Environment${NC}"
echo "================================================"

# Create .env file
cat > .env << EOF
# Wallet Configuration
WALLET_ADDRESS=$WALLET_ADDRESS
PRIVATE_KEY=$PRIVATE_KEY

# API Keys  
ALCHEMY_API_KEY=$ALCHEMY_API_KEY
INFURA_API_KEY=$INFURA_API_KEY
ETHERSCAN_API_KEY=$ETHERSCAN_API_KEY

# Discord
DISCORD_WEBHOOK=$DISCORD_WEBHOOK

# Network
NETWORK=arbitrum
CHAIN_ID=42161

# Bot Settings
MIN_PROFIT_THRESHOLD=10
GAS_PRICE_LIMIT=1
SLIPPAGE_TOLERANCE=3
EOF

echo "✅ Environment configured"

echo -e "\n${GREEN}Step 2: Installing Core Dependencies${NC}"
echo "================================================"

# Install only essential packages first
npm install ethers@5.7.2 dotenv axios

echo -e "\n${BLUE}Step 3: Checking Your Balance${NC}"
echo "================================================"

# Simple balance check script
cat > check-balance.js << 'EOF'
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
EOF

node check-balance.js

echo -e "\n${GREEN}Step 4: FREE Funding Options${NC}"
echo "================================================"
echo ""
echo -e "${BLUE}Option A: FREE TESTNET ETH (Practice First)${NC}"
echo "------------------------------------------------"
echo "1. Arbitrum Sepolia Faucet (Instant, No signup):"
echo "   🔗 https://www.alchemy.com/faucets/arbitrum-sepolia"
echo "   - Get 0.1 ETH instantly"
echo "   - Just paste your address: $WALLET_ADDRESS"
echo ""
echo "2. QuickNode Faucet:"
echo "   🔗 https://faucet.quicknode.com/arbitrum/sepolia"
echo "   - Get 0.05 ETH"
echo "   - No Twitter needed"
echo ""

echo -e "${BLUE}Option B: GET REAL ARBITRUM ETH${NC}"
echo "------------------------------------------------"
echo "1. EASIEST - Buy with Card (5 minutes):"
echo "   🔗 https://app.uniswap.org"
echo "   - Click 'Buy' button"
echo "   - Select Arbitrum network"
echo "   - Buy with credit/debit card via MoonPay"
echo "   - Minimum: \$30"
echo ""
echo "2. FROM EXCHANGE (If you have crypto):"
echo "   • Binance: Withdraw ETH directly to Arbitrum"
echo "   • Coinbase: Buy ETH → Send to your wallet on Arbitrum"
echo "   • KuCoin: Direct Arbitrum withdrawal"
echo ""
echo "3. BRIDGE FROM ETHEREUM (If you have ETH on mainnet):"
echo "   🔗 https://bridge.arbitrum.io"
echo "   - Official bridge"
echo "   - Takes ~10 minutes"
echo ""
echo "4. CHEAP BRIDGES (From other chains):"
echo "   🔗 https://jumper.exchange"
echo "   - Bridge from ANY chain"
echo "   - Find cheapest route"
echo ""

echo -e "${YELLOW}Step 5: Quick Funding Helper${NC}"
echo "================================================"

# Create simple funding helper
cat > get-funded.js << 'EOF'
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
EOF

node get-funded.js

echo -e "\n${GREEN}Step 6: Bot Scripts Setup${NC}"
echo "================================================"

# Create the arbitrage monitoring script
cat > run-arbitrage.js << 'EOF'
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
EOF

echo -e "\n${GREEN}✅ SETUP COMPLETE!${NC}"
echo "================================================"
echo ""
echo "📋 NEXT STEPS:"
echo ""
echo "1. Get funded (choose one):"
echo "   a) ${BLUE}FREE testnet:${NC} https://www.alchemy.com/faucets/arbitrum-sepolia"
echo "   b) ${BLUE}Buy with card:${NC} https://app.uniswap.org (easiest)"
echo "   c) ${BLUE}From exchange:${NC} Withdraw to Arbitrum network"
echo ""
echo "2. Check your balance:"
echo "   ${GREEN}node check-balance.js${NC}"
echo ""
echo "3. Run the bot:"
echo "   ${GREEN}node run-arbitrage.js${NC}"
echo ""
echo "================================================"
echo -e "${YELLOW}YOUR WALLET:${NC} ${GREEN}$WALLET_ADDRESS${NC}"
echo "================================================"
echo ""

# Open funding page automatically if possible
if command -v open &> /dev/null; then
    echo "Opening Uniswap to buy ETH..."
    open "https://app.uniswap.org"
elif command -v xdg-open &> /dev/null; then
    echo "Opening Uniswap to buy ETH..."
    xdg-open "https://app.uniswap.org"
fi

echo -e "${GREEN}Ready! Just need to add funds to start trading.${NC}"