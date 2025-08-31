import torch
import torch.nn as nn
import numpy as np
from typing import Dict, List
import redis
import json

class ArbitrageOptimizer(nn.Module):
    def __init__(self, input_dim=20, hidden_dim=128):
        super().__init__()
        
        # Use Metal Performance Shaders on M1 Mac
        self.device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
        
        self.network = nn.Sequential(
            nn.Linear(input_dim, hidden_dim),
            nn.ReLU(),
            nn.Dropout(0.2),
            nn.Linear(hidden_dim, 64),
            nn.ReLU(),
            nn.Linear(64, 3)  # [execute_prob, gas_price, slippage]
        ).to(self.device)
        
        self.optimizer = torch.optim.Adam(self.parameters(), lr=0.001)
        self.redis_client = redis.Redis(host='localhost', port=6379)
        
    def forward(self, x):
        return self.network(x)
    
    def predict_opportunity(self, features: Dict) -> Dict:
        """Predict if an opportunity should be executed"""
        
        # Convert features to tensor
        x = self.features_to_tensor(features).to(self.device)
        
        with torch.no_grad():
            output = self.forward(x)
            
        execute_prob = torch.sigmoid(output[0]).item()
        optimal_gas = output[1].item() * 100  # Scale to gwei
        max_slippage = torch.sigmoid(output[2]).item() * 5  # 0-5% slippage
        
        return {
            'execute': execute_prob > 0.7,
            'confidence': execute_prob,
            'gas_price_gwei': optimal_gas,
            'max_slippage': max_slippage
        }
    
    def features_to_tensor(self, features: Dict) -> torch.Tensor:
        """Convert opportunity features to model input"""
        
        return torch.tensor([
            features['spread_percent'],
            features['volume_usd'],
            features['gas_price_gwei'],
            features['block_number'] % 1000,
            features['mempool_density'],
            features['volatility_1h'],
            features['dex_liquidity'],
            # ... more features
        ], dtype=torch.float32)
    
    def train_on_results(self, results: List[Dict]):
        """Online learning from execution results"""
        
        for result in results:
            features = self.features_to_tensor(result['features'])
            
            # Create target based on actual profit
            actual_profit = result['actual_profit']
            success = 1.0 if actual_profit > 0 else 0.0
            
            target = torch.tensor([
                success,
                result['gas_used'] / 1e9,
                result['slippage']
            ], dtype=torch.float32).to(self.device)
            
            # Train step
            self.optimizer.zero_grad()
            output = self.forward(features)
            loss = nn.MSELoss()(output, target)
            loss.backward()
            self.optimizer.step()
            
            # Log to Redis
            self.redis_client.rpush('training_history', json.dumps({
                'timestamp': result['timestamp'],
                'loss': loss.item(),
                'profit': actual_profit
            }))

class MEVPredictor:
    """Predict MEV competition and optimal timing"""
    
    def __init__(self):
        self.model = ArbitrageOptimizer()
        self.load_pretrained()
        
    def should_execute_now(self, opportunity: Dict) -> bool:
        """Decide if we should execute immediately or wait"""
        
        # Get current mempool state
        mempool_congestion = self.get_mempool_congestion()
        
        # Predict competition
        competition_level = self.predict_competition(opportunity)
        
        # Optimal timing
        if competition_level > 0.8:
            return True  # Execute immediately
        elif mempool_congestion < 0.3:
            return True  # Low congestion, execute
        else:
            return False  # Wait for better conditions
    
    def predict_competition(self, opp: Dict) -> float:
        """Predict how many other bots will compete"""
        
        features = {
            'profit_usd': opp['profit'],
            'token_popularity': self.get_token_popularity(opp['tokens']),
            'dex_volume': opp['dex_volume'],
            'spread': opp['spread']
        }
        
        return self.model.predict_opportunity(features)['confidence']