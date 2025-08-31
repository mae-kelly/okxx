# Production Flash Loan Arbitrage Bot

A production-ready DeFi arbitrage bot that monitors real DEX prices and executes profitable trades using Aave V3 flash loans.

## Features

- ✅ Real DEX price monitoring (Uniswap, Sushiswap)
- ✅ Aave V3 flash loan integration
- ✅ Gas price optimization
- ✅ MEV protection via Flashbots RPC
- ✅ Discord notifications
- ✅ Automatic profit calculation
- ✅ Multiple token pair support
- ✅ Production logging

## Requirements

- Node.js v16+
- Ethereum wallet with ~0.5 ETH for gas
- Alchemy API key
- Etherscan API key (for verification)
- Discord webhook (optional)

## Installation

1. Clone and install:
```bash
git clone <repository>
cd flash-loan-arbitrage
npm install