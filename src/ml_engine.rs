use std::sync::Arc;
use anyhow::Result;
use ndarray::{Array1, Array2, s};
use ndarray_rand::RandomExt;
use ndarray_rand::rand_distr::Uniform;
use parking_lot::RwLock;
use crate::types::ArbitrageOpportunity;
use rust_decimal::prelude::ToPrimitive;

// Simple neural network without serialization issues
pub struct MetalMLEngine {
    layers: Arc<RwLock<Vec<LayerWeights>>>,
    learning_rate: f32,
    epochs: usize,
}

// Store weights as vectors instead of ndarray for serialization
struct LayerWeights {
    weights: Vec<Vec<f32>>,
    bias: Vec<f32>,
    input_size: usize,
    output_size: usize,
}

impl MetalMLEngine {
    pub fn new() -> Self {
        let layers = vec![
            LayerWeights::new(10, 64),
            LayerWeights::new(64, 32),
            LayerWeights::new(32, 16),
            LayerWeights::new(16, 1),
        ];
        
        Self {
            layers: Arc::new(RwLock::new(layers)),
            learning_rate: 0.001,
            epochs: 100,
        }
    }
    
    pub async fn train(&self, data: &[ArbitrageOpportunity]) -> Result<()> {
        if data.len() < 10 {
            return Ok(());
        }
        
        let features = self.extract_features(data);
        let targets = self.extract_targets(data);
        
        // Simple training loop
        for _ in 0..self.epochs {
            for (feature, target) in features.iter().zip(targets.iter()) {
                self.forward_backward(feature, *target);
            }
        }
        
        Ok(())
    }
    
    pub async fn predict(&self, opportunity: &ArbitrageOpportunity) -> f64 {
        let features = self.extract_single_features(opportunity);
        self.forward(&features)
    }
    
    fn extract_features(&self, data: &[ArbitrageOpportunity]) -> Vec<Vec<f32>> {
        data.iter().map(|opp| self.extract_single_features(opp)).collect()
    }
    
    fn extract_single_features(&self, opp: &ArbitrageOpportunity) -> Vec<f32> {
        vec![
            opp.initial_amount.to_f32().unwrap_or(0.0),
            opp.roi_percentage as f32,
            opp.path.len() as f32,
            opp.total_gas_cost.to_f32().unwrap_or(0.0),
            opp.flash_loan_fee.to_f32().unwrap_or(0.0),
            opp.timestamp.timestamp() as f32 / 1_000_000.0,
            match opp.chain {
                crate::types::Chain::Ethereum => 1.0,
                crate::types::Chain::BinanceSmartChain => 2.0,
                crate::types::Chain::Polygon => 3.0,
                crate::types::Chain::Arbitrum => 4.0,
                crate::types::Chain::Optimism => 5.0,
                _ => 0.0,
            },
            opp.execution_time_ms as f32,
            opp.ml_confidence as f32,
            if opp.profit_usd > 0.0 { 1.0 } else { 0.0 },
        ]
    }
    
    fn extract_targets(&self, data: &[ArbitrageOpportunity]) -> Vec<f32> {
        data.iter().map(|opp| opp.profit_usd as f32).collect()
    }
    
    fn forward(&self, input: &[f32]) -> f64 {
        let layers = self.layers.read();
        let mut current = input.to_vec();
        
        for layer in layers.iter() {
            current = layer.forward(&current);
        }
        
        current[0] as f64
    }
    
    fn forward_backward(&self, input: &[f32], target: f32) {
        let mut layers = self.layers.write();
        let mut activations = vec![input.to_vec()];
        let mut current = input.to_vec();
        
        // Forward pass
        for layer in layers.iter() {
            current = layer.forward(&current);
            activations.push(current.clone());
        }
        
        // Backward pass (simplified)
        let output = current[0];
        let error = output - target;
        
        // Update weights (simplified gradient descent)
        for (i, layer) in layers.iter_mut().enumerate().rev() {
            let input_activation = &activations[i];
            layer.update_weights(error * self.learning_rate, input_activation);
        }
    }
    
    pub fn get_feature_importance(&self) -> Vec<f32> {
        let layers = self.layers.read();
        if let Some(first_layer) = layers.first() {
            // Sum absolute weights for each input feature
            let mut importance = vec![0.0f32; first_layer.input_size];
            for (i, weights_row) in first_layer.weights.iter().enumerate() {
                for (j, weight) in weights_row.iter().enumerate() {
                    if j < importance.len() {
                        importance[j] += weight.abs();
                    }
                }
            }
            importance
        } else {
            vec![]
        }
    }
}

impl LayerWeights {
    fn new(input_size: usize, output_size: usize) -> Self {
        let mut weights = vec![vec![0.0f32; input_size]; output_size];
        let mut bias = vec![0.0f32; output_size];
        
        // Initialize with small random values
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let range = (6.0 / (input_size + output_size) as f32).sqrt();
        
        for i in 0..output_size {
            for j in 0..input_size {
                weights[i][j] = rng.gen_range(-range..range);
            }
            bias[i] = rng.gen_range(-range..range);
        }
        
        Self {
            weights,
            bias,
            input_size,
            output_size,
        }
    }
    
    fn forward(&self, input: &[f32]) -> Vec<f32> {
        let mut output = vec![0.0f32; self.output_size];
        
        for i in 0..self.output_size {
            let mut sum = self.bias[i];
            for j in 0..input.len().min(self.input_size) {
                sum += input[j] * self.weights[i][j];
            }
            // ReLU activation
            output[i] = sum.max(0.0);
        }
        
        output
    }
    
    fn update_weights(&mut self, error: f32, input: &[f32]) {
        // Simplified weight update
        for i in 0..self.output_size {
            for j in 0..input.len().min(self.input_size) {
                self.weights[i][j] -= error * input[j] * 0.001;
            }
            self.bias[i] -= error * 0.001;
        }
    }
}