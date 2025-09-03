#!/bin/bash

# Start script for arbitrage bot
set -e

echo "🚀 Starting Arbitrage Bot..."

# Check if .env exists
if [ ! -f .env ]; then
    echo "❌ Error: .env file not found!"
    echo "Copy .env.example to .env and fill in your values"
    exit 1
fi

# Create logs directory
mkdir -p logs

# Check if wallet has funds
source .env
if [ -z "$PRIVATE_KEY" ]; then
    echo "❌ Error: PRIVATE_KEY not set in .env"
    exit 1
fi

# Build in release mode
echo "📦 Building in release mode..."
cargo build --release

# Run with logging
echo "🏃 Starting bot..."
RUST_LOG=info ./target/release/arb-scanner 2>&1 | tee -a logs/bot.log

