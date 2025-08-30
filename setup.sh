#!/bin/bash

# Setup script for Arbitrage Scanner
set -e

echo "🚀 Setting up Arbitrage Scanner..."

# Create necessary directories
mkdir -p src/exchanges
mkdir -p data
mkdir -p logs
mkdir -p models

# Check if src/exchanges/mod.rs exists and add okx module
if [ -f "src/exchanges/mod.rs" ]; then
    # Check if okx module is already declared
    if ! grep -q "pub mod okx;" src/exchanges/mod.rs; then
        echo "pub mod okx;" >> src/exchanges/mod.rs
        echo "✅ Added OKX module to exchanges"
    fi
fi

# Create .gitignore if it doesn't exist
if [ ! -f .gitignore ]; then
    cat > .gitignore << 'EOF'
# Environment files
.env
.env.production
.env.local
*.env

# Keys and secrets
*.key
*.pem
private_key.txt

# Build artifacts
target/
Cargo.lock

# IDE
.idea/
.vscode/
*.swp
*.swo

# Logs
logs/
*.log

# Data
data/*.db
data/*.log

# OS
.DS_Store
Thumbs.db
EOF
    echo "✅ Created .gitignore"
fi

# Build the project
echo "🔨 Building project..."
cargo build --release

echo ""
echo "✅ Setup complete!"
echo ""
echo "Next steps:"
echo "1. Create your .env file:"
echo "   cp .env.production .env"
echo ""
echo "2. Add your OKX passphrase to .env:"
echo "   echo 'OKX_PASSPHRASE=YourPassphrase' >> .env"
echo ""
echo "3. Run the scanner:"
echo "   cargo run --release"
echo ""
echo "4. Monitor the output and check Discord for notifications!"