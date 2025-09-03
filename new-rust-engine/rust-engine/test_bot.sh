#!/bin/bash

echo "🧪 Testing Arbitrage Bot Setup..."

# Test compilation
echo "Testing compilation..."
cargo check

# Run tests
echo "Running tests..."
cargo test

# Test wallet generation
echo "Testing wallet generation..."
cargo run --bin generate_wallet

echo "✅ All tests passed!"
