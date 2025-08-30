#!/bin/bash

# Arbitrage Scanner Run Script
# Usage: ./run.sh [debug|release|test]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}🚀 Crypto Arbitrage Scanner Launcher${NC}"
echo "====================================="

# Check if .env exists
if [ ! -f .env ]; then
    if [ -f .env.template ]; then
        echo -e "${YELLOW}⚠️  .env file not found. Creating from template...${NC}"
        cp .env.template .env
        echo -e "${GREEN}✅ Created .env file. Please edit it with your API keys.${NC}"
    else
        echo -e "${RED}❌ No .env or .env.template found!${NC}"
        exit 1
    fi
fi

# Load environment variables
export $(cat .env | grep -v '^#' | xargs)

# Parse command line arguments
MODE=${1:-release}

case $MODE in
    debug)
        echo -e "${YELLOW}🐛 Running in DEBUG mode with verbose logging${NC}"
        export DEBUG=true
        export RUST_LOG=debug
        cargo run
        ;;
    
    release)
        echo -e "${GREEN}⚡ Running in RELEASE mode (optimized)${NC}"
        export DEBUG=false
        export RUST_LOG=info
        cargo run --release
        ;;
    
    test)
        echo -e "${YELLOW}🧪 Running in TEST mode with simulated data${NC}"
        export DEBUG=true
        export RUST_LOG=debug
        export TEST_MODE=true
        cargo run
        ;;
    
    build)
        echo -e "${GREEN}🔨 Building optimized release binary${NC}"
        cargo build --release
        echo -e "${GREEN}✅ Binary built at: target/release/arbitrage-scanner${NC}"
        ;;
    
    check)
        echo -e "${YELLOW}🔍 Checking system requirements...${NC}"
        
        # Check Rust
        if command -v rustc &> /dev/null; then
            echo -e "${GREEN}✅ Rust installed: $(rustc --version)${NC}"
        else
            echo -e "${RED}❌ Rust not installed. Install from https://rustup.rs${NC}"
            exit 1
        fi
        
        # Check Cargo
        if command -v cargo &> /dev/null; then
            echo -e "${GREEN}✅ Cargo installed: $(cargo --version)${NC}"
        else
            echo -e "${RED}❌ Cargo not installed${NC}"
            exit 1
        fi
        
        # Check Python (for ML)
        if command -v python3 &> /dev/null; then
            echo -e "${GREEN}✅ Python3 installed: $(python3 --version)${NC}"
        else
            echo -e "${YELLOW}⚠️  Python3 not installed (optional for ML)${NC}"
        fi
        
        # Test RPC endpoints
        echo -e "${YELLOW}🌐 Testing RPC endpoints...${NC}"
        
        # Test Ethereum RPC
        response=$(curl -s -X POST -H "Content-Type: application/json" \
            --data '{"jsonrpc":"2.0","method":"eth_blockNumber","params":[],"id":1}' \
            ${ETH_RPC_URL:-https://eth.llamarpc.com} | grep -o '"result"')
        
        if [ "$response" == '"result"' ]; then
            echo -e "${GREEN}✅ Ethereum RPC working${NC}"
        else
            echo -e "${RED}❌ Ethereum RPC not responding${NC}"
        fi
        
        # Check metrics server
        echo -e "${YELLOW}📊 Checking if metrics server is available...${NC}"
        if curl -s http://localhost:8080/health > /dev/null 2>&1; then
            echo -e "${YELLOW}⚠️  Metrics server already running on port 8080${NC}"
        else
            echo -e "${GREEN}✅ Port 8080 available for metrics${NC}"
        fi
        
        echo -e "${GREEN}✅ System check complete!${NC}"
        ;;
    
    monitor)
        echo -e "${GREEN}📊 Opening monitoring dashboard...${NC}"
        echo "Metrics URL: http://localhost:8080/metrics"
        echo "Health URL: http://localhost:8080/health"
        
        # Start the scanner in background
        export DEBUG=false
        export RUST_LOG=info
        cargo run --release &
        SCANNER_PID=$!
        
        echo -e "${GREEN}Scanner running with PID: $SCANNER_PID${NC}"
        echo "Press Ctrl+C to stop monitoring"
        
        # Wait a bit for server to start
        sleep 5
        
        # Open browser if available
        if command -v xdg-open &> /dev/null; then
            xdg-open http://localhost:8080/metrics
        elif command -v open &> /dev/null; then
            open http://localhost:8080/metrics
        fi
        
        # Monitor logs
        tail -f logs/arbitrage.log 2>/dev/null || \
        (echo "Creating logs directory..." && mkdir -p logs && touch logs/arbitrage.log && tail -f logs/arbitrage.log)
        ;;
    
    clean)
        echo -e "${YELLOW}🧹 Cleaning build artifacts and data...${NC}"
        cargo clean
        rm -rf data/*.log data/*.bak
        echo -e "${GREEN}✅ Cleaned${NC}"
        ;;
    
    *)
        echo "Usage: $0 [debug|release|test|build|check|monitor|clean]"
        echo ""
        echo "Commands:"
        echo "  debug    - Run with debug logging enabled"
        echo "  release  - Run optimized release build (default)"
        echo "  test     - Run with test data generation"
        echo "  build    - Build optimized binary"
        echo "  check    - Check system requirements"
        echo "  monitor  - Run with monitoring dashboard"
        echo "  clean    - Clean build artifacts"
        exit 1
        ;;
esac