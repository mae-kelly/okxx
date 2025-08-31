#!/usr/bin/env python3
import sys
import json
import pandas as pd
import numpy as np
from datetime import datetime
import warnings
warnings.filterwarnings('ignore')

# M1 Mac optimized imports
import torch
import torch.nn as nn
import torch.optim as optim

# Check for M1 GPU (Metal Performance Shaders)
if torch.backends.mps.is_available():
    device = torch.device("mps")
    print("🎯 Using M1 GPU (Metal Performance Shaders)", file=sys.stderr)
else:
    device = torch.device("cpu")
    print("⚠️ M1 GPU not available, using CPU", file=sys.stderr)

class ArbitragePredictor(nn.Module):
    def __init__(self, input_features=10):
        super(ArbitragePredictor, self).__init__()
        self.fc1 = nn.Linear(input_features, 128)
        self.fc2 = nn.Linear(128, 64)
        self.fc3 = nn.Linear(64, 32)
        self.fc4 = nn.Linear(32, 3)  # 3 outputs: success_prob, profit_mult, priority
        self.dropout = nn.Dropout(0.2)
        
    def forward(self, x):
        x = torch.relu(self.fc1(x))
        x = self.dropout(x)
        x = torch.relu(self.fc2(x))
        x = self.dropout(x)
        x = torch.relu(self.fc3(x))
        return self.fc4(x)

class ArbitrageML:
    def __init__(self):
        self.model = ArbitragePredictor().to(device)
        self.optimizer = optim.Adam(self.model.parameters(), lr=0.001)
        self.criterion = nn.MSELoss()
        self.data_buffer = []
        self.load_historical_data()
        
    def load_historical_data(self):
        try:
            self.df = pd.read_csv('../data/opportunities.csv')
            print(f"📊 Loaded {len(self.df)} historical records", file=sys.stderr)
        except:
            self.df = pd.DataFrame()
            
    def preprocess_opportunity(self, opp):
        """Convert opportunity to feature vector"""
        features = [
            opp.get('spread', 0),
            opp.get('grossProfit', 0),
            opp.get('flashLoanFee', 0),
            opp.get('gasCost', 0),
            opp.get('netProfit', 0),
            1 if opp.get('chain') == 'ethereum' else 0,
            1 if opp.get('chain') == 'arbitrum' else 0,
            1 if 'uniswap' in opp.get('dex1', '').lower() else 0,
            1 if 'uniswap' in opp.get('dex2', '').lower() else 0,
            datetime.now().hour / 24  # Time of day feature
        ]
        return torch.tensor(features, dtype=torch.float32).to(device)
    
    def predict(self, opportunities):
        """Predict success probability and optimal execution"""
        self.model.eval()
        predictions = []
        
        with torch.no_grad():
            for opp in opportunities:
                features = self.preprocess_opportunity(opp)
                output = self.model(features.unsqueeze(0))
                
                success_prob = torch.sigmoid(output[0, 0]).item()
                profit_multiplier = torch.relu(output[0, 1]).item()
                priority = torch.sigmoid(output[0, 2]).item()
                
                predictions.append({
                    'opportunity': opp,
                    'success_probability': success_prob,
                    'expected_profit': opp['netProfit'] * profit_multiplier,
                    'priority_score': priority * 100
                })
        
        # Sort by priority score
        predictions.sort(key=lambda x: x['priority_score'], reverse=True)
        return predictions
    
    def train_on_batch(self, batch_data):
        """Online learning from recent results"""
        if len(batch_data) < 10:
            return
            
        self.model.train()
        
        for data in batch_data:
            features = self.preprocess_opportunity(data)
            
            # Create target based on actual results
            success = 1.0 if data.get('success', False) else 0.0
            profit_ratio = data.get('actual_profit', 0) / max(data.get('netProfit', 1), 1)
            priority = min(data.get('netProfit', 0) / 100, 1.0)
            
            target = torch.tensor([success, profit_ratio, priority], dtype=torch.float32).to(device)
            
            # Forward pass
            output = self.model(features.unsqueeze(0))
            loss = self.criterion(output[0], target)
            
            # Backward pass
            self.optimizer.zero_grad()
            loss.backward()
            self.optimizer.step()
    
    def recommend_action(self, opportunities):
        """Generate actionable recommendations"""
        if not opportunities:
            return None
            
        predictions = self.predict(opportunities)
        
        # Get top recommendation
        if predictions and predictions[0]['priority_score'] > 50:
            top = predictions[0]
            opp = top['opportunity']
            
            recommendation = {
                'action': 'EXECUTE',
                'chain': opp['chain'],
                'route': f"{opp['dex1']} → {opp['dex2']}",
                'pair': f"{opp['token0']}/{opp['token1']}",
                'expected_profit': f"${top['expected_profit']:.2f}",
                'confidence': f"{top['success_probability']*100:.1f}%",
                'priority': f"{top['priority_score']:.0f}/100"
            }
            
            return recommendation
        
        return {'action': 'WAIT', 'reason': 'No high-confidence opportunities'}
    
    def process_stream(self):
        """Process incoming opportunity stream"""
        for line in sys.stdin:
            try:
                opportunities = json.loads(line.strip())
                
                if opportunities:
                    # Make recommendation
                    recommendation = self.recommend_action(opportunities)
                    
                    if recommendation:
                        output = f"ACTION: {recommendation['action']}"
                        if recommendation['action'] == 'EXECUTE':
                            output += f" | {recommendation['chain']} | {recommendation['route']} | {recommendation['pair']}"
                            output += f" | Profit: {recommendation['expected_profit']} | Confidence: {recommendation['confidence']}"
                        print(output)
                        sys.stdout.flush()
                    
                    # Add to buffer for training
                    self.data_buffer.extend(opportunities[:5])
                    
                    # Train periodically
                    if len(self.data_buffer) >= 50:
                        self.train_on_batch(self.data_buffer[-50:])
                        self.data_buffer = self.data_buffer[-100:]  # Keep last 100
                        
            except Exception as e:
                print(f"Error: {e}", file=sys.stderr)

if __name__ == "__main__":
    ml = ArbitrageML()
    ml.process_stream()