#!/bin/bash

echo "🛑 Stopping Arbitrage Bot..."
pkill -f arb-scanner || echo "Bot was not running"
echo "✅ Bot stopped"
