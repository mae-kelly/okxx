#!/bin/bash

# Install Node dependencies
npm install

# Install Python dependencies for M1 Mac
pip3 install torch torchvision torchaudio
pip3 install pandas numpy scikit-learn
pip3 install matplotlib seaborn

# Create data directories
mkdir -p data/models
mkdir -p logs

# Initialize CSV
echo "timestamp,chain,dex1,dex2,token0,token1,amount,spread,profit_usd,gas_cost,net_profit,executed,success" > data/opportunities.csv

echo "✅ Installation complete!"
echo "Run 'npm start' to begin scanning"