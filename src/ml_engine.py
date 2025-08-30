#!/usr/bin/env python3
"""
Metal-optimized ML Engine for Crypto Arbitrage Scanner
Utilizes Apple M1/M2/M3 GPU via Metal Performance Shaders
"""

import torch
import torch.nn as nn
import torch.nn.functional as F
import numpy as np
from typing import List, Dict, Tuple, Optional
import json
import asyncio
import websockets
from dataclasses import dataclass
from datetime import datetime
import logging

# Configure for Apple Silicon
if torch.backends.mps.is_available():
    device = torch.device("mps")
    print("✅ Using Apple Silicon GPU (Metal Performance Shaders)")
else:
    device = torch.device("cpu")
    print("⚠️ MPS not available, using CPU")

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)


@dataclass
class ArbitrageFeatures:
    """Features extracted from arbitrage opportunity"""
    initial_amount: float
    roi_percentage: float
    path_length: float
    gas_cost: float
    flash_loan_fee: float
    hour: float
    day_of_week: float
    chain_id: float
    execution_time: float
    volume_ratio: float
    price_spread: float
    liquidity_depth: float


class MetalOptimizedNN(nn.Module):
    """Neural network optimized for Metal Performance Shaders"""
    
    def __init__(self, input_dim=12, hidden_dims=[256, 128, 64, 32], output_dim=1):
        super().__init__()
        
        # Build layers dynamically
        self.layers = nn.ModuleList()
        prev_dim = input_dim
        
        for hidden_dim in hidden_dims:
            self.layers.append(nn.Linear(prev_dim, hidden_dim))
            self.layers.append(nn.BatchNorm1d(hidden_dim))
            self.layers.append(nn.ReLU(inplace=True))
            self.layers.append(nn.Dropout(0.2))
            prev_dim = hidden_dim
        
        # Output layer
        self.output = nn.Linear(prev_dim, output_dim)
        
        # Move to MPS device
        self.to(device)
        
        # Initialize weights using Kaiming initialization for ReLU
        self.apply(self._init_weights)
    
    def _init_weights(self, module):
        if isinstance(module, nn.Linear):
            nn.init.kaiming_normal_(module.weight, mode='fan_out', nonlinearity='relu')
            if module.bias is not None:
                nn.init.constant_(module.bias, 0)
    
    def forward(self, x):
        """Optimized forward pass for M1 GPU"""
        for layer in self.layers:
            x = layer(x)
        return self.output(x)


class ArbitrageMLEngine:
    """Main ML Engine for arbitrage opportunity analysis"""
    
    def __init__(self):
        self.model = MetalOptimizedNN()
        self.optimizer = torch.optim.AdamW(
            self.model.parameters(), 
            lr=0.001,
            weight_decay=0.01
        )
        self.scheduler = torch.optim.lr_scheduler.CosineAnnealingWarmRestarts(
            self.optimizer, T_0=10, T_mult=2
        )
        self.scaler = torch.amp.GradScaler(enabled=False)  # For future compatibility
        # Training history
        self.loss_history = []
        self.validation_scores = []
        
    def extract_features(self, opportunity: Dict) -> torch.Tensor:
        """Extract and normalize features from opportunity data"""
        features = [
            opportunity.get('initial_amount', 0) / 10000,
            opportunity.get('roi_percentage', 0) / 100,
            opportunity.get('path_length', 1) / 5,
            opportunity.get('gas_cost', 0) / 1000,
            opportunity.get('flash_loan_fee', 0) / 100,
            opportunity.get('hour', 0) / 24,
            opportunity.get('day_of_week', 0) / 7,
            opportunity.get('chain_id', 1) / 10,
            opportunity.get('execution_time', 0) / 1000,
            opportunity.get('volume_ratio', 1),
            opportunity.get('price_spread', 0) / 10,
            opportunity.get('liquidity_depth', 0) / 1000000,
        ]
        
        return torch.tensor(features, dtype=torch.float32, device=device)
    
    def train_batch(self, opportunities: List[Dict], targets: List[float]):
        """Train on a batch of opportunities using M1 GPU"""
        self.model.train()
        
        # Prepare batch data
        batch_features = torch.stack([
            self.extract_features(opp) for opp in opportunities
        ])
        batch_targets = torch.tensor(targets, dtype=torch.float32, device=device).unsqueeze(1)
        
        # Forward pass
        predictions = self.model(batch_features)
        loss = F.mse_loss(predictions, batch_targets)
        
        # Backward pass (optimized for Metal)
        self.optimizer.zero_grad(set_to_none=True)  # More efficient than zero_grad()
        loss.backward()
        
        # Gradient clipping for stability
        torch.nn.utils.clip_grad_norm_(self.model.parameters(), max_norm=1.0)
        
        self.optimizer.step()
        self.scheduler.step()
        
        self.loss_history.append(loss.item())
        return loss.item()
    
    @torch.no_grad()
    def predict(self, opportunity: Dict) -> float:
        """Predict profitability score for an opportunity"""
        self.model.eval()
        features = self.extract_features(opportunity)
        
        # Add batch dimension
        features = features.unsqueeze(0)
        
        prediction = self.model(features)
        return prediction.item()
    
    @torch.no_grad()
    def batch_predict(self, opportunities: List[Dict]) -> List[float]:
        """Batch prediction for multiple opportunities"""
        self.model.eval()
        
        if not opportunities:
            return []
        
        batch_features = torch.stack([
            self.extract_features(opp) for opp in opportunities
        ])
        
        predictions = self.model(batch_features)
        return predictions.squeeze().tolist()
    
    def calculate_feature_importance(self) -> Dict[str, float]:
        """Calculate feature importance using gradient-based method"""
        self.model.eval()
        
        # Create a sample input
        sample_input = torch.randn(1, 12, device=device, requires_grad=True)
        
        # Forward pass
        output = self.model(sample_input)
        
        # Calculate gradients
        output.backward()
        
        # Get feature importance from gradients
        importance = sample_input.grad.abs().mean(dim=0).cpu().numpy()
        
        feature_names = [
            'initial_amount', 'roi_percentage', 'path_length', 'gas_cost',
            'flash_loan_fee', 'hour', 'day_of_week', 'chain_id',
            'execution_time', 'volume_ratio', 'price_spread', 'liquidity_depth'
        ]
        
        return dict(zip(feature_names, importance))
    
    def save_model(self, path: str):
        """Save model checkpoint"""
        torch.save({
            'model_state_dict': self.model.state_dict(),
            'optimizer_state_dict': self.optimizer.state_dict(),
            'scheduler_state_dict': self.scheduler.state_dict(),
            'loss_history': self.loss_history,
            'validation_scores': self.validation_scores,
        }, path)
        logger.info(f"Model saved to {path}")
    
    def load_model(self, path: str):
        """Load model checkpoint"""
        checkpoint = torch.load(path, map_location=device)
        self.model.load_state_dict(checkpoint['model_state_dict'])
        self.optimizer.load_state_dict(checkpoint['optimizer_state_dict'])
        self.scheduler.load_state_dict(checkpoint['scheduler_state_dict'])
        self.loss_history = checkpoint.get('loss_history', [])
        self.validation_scores = checkpoint.get('validation_scores', [])
        logger.info(f"Model loaded from {path}")


class MLWebSocketServer:
    """WebSocket server for Rust integration"""
    
    def __init__(self, engine: ArbitrageMLEngine, host='127.0.0.1', port=8765):
        self.engine = engine
        self.host = host
        self.port = port
    
    async def handle_client(self, websocket, path):
        """Handle incoming WebSocket connections from Rust"""
        try:
            async for message in websocket:
                data = json.loads(message)
                
                if data['type'] == 'train':
                    # Training request
                    opportunities = data['opportunities']
                    targets = data['targets']
                    loss = self.engine.train_batch(opportunities, targets)
                    
                    response = {
                        'type': 'train_result',
                        'loss': loss,
                        'timestamp': datetime.now().isoformat()
                    }
                    
                elif data['type'] == 'predict':
                    # Prediction request
                    opportunity = data['opportunity']
                    score = self.engine.predict(opportunity)
                    
                    response = {
                        'type': 'prediction',
                        'score': score,
                        'confidence': min(abs(score) / 100, 1.0),
                        'timestamp': datetime.now().isoformat()
                    }
                    
                elif data['type'] == 'batch_predict':
                    # Batch prediction request
                    opportunities = data['opportunities']
                    scores = self.engine.batch_predict(opportunities)
                    
                    response = {
                        'type': 'batch_prediction',
                        'scores': scores,
                        'timestamp': datetime.now().isoformat()
                    }
                    
                elif data['type'] == 'feature_importance':
                    # Feature importance request
                    importance = self.engine.calculate_feature_importance()
                    
                    response = {
                        'type': 'feature_importance',
                        'importance': importance,
                        'timestamp': datetime.now().isoformat()
                    }
                
                else:
                    response = {
                        'type': 'error',
                        'message': f'Unknown request type: {data["type"]}'
                    }
                
                await websocket.send(json.dumps(response))
                
        except Exception as e:
            logger.error(f"WebSocket error: {e}")
            error_response = {
                'type': 'error',
                'message': str(e)
            }
            await websocket.send(json.dumps(error_response))
    
    async def start(self):
        """Start the WebSocket server"""
        logger.info(f"Starting ML WebSocket server on {self.host}:{self.port}")
        async with websockets.serve(self.handle_client, self.host, self.port):
            await asyncio.Future()  # Run forever


def main():
    """Main entry point"""
    # Initialize ML engine
    engine = ArbitrageMLEngine()
    
    # Initialize WebSocket server
    server = MLWebSocketServer(engine)
    
    # Run the server
    try:
        asyncio.run(server.start())
    except KeyboardInterrupt:
        logger.info("Shutting down ML engine...")
        engine.save_model("model_checkpoint.pt")


if __name__ == "__main__":
    main()