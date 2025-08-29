#!/bin/bash

echo "Fixing compilation warnings..."

# Fix unused imports
echo "Removing unused imports..."
sed -i '' '/use std::collections::HashMap;/d' src/types.rs
sed -i '' '/use dashmap::DashMap;/d' src/arbitrage.rs
sed -i '' '/use smartcore::tree::decision_tree_regressor::DecisionTreeRegressor;/d' src/ml.rs
sed -i '' 's/use ndarray::{Array2, ArrayView1, s};/use ndarray::{Array2, s};/' src/ml.rs
sed -i '' '/use std::borrow::Cow;/d' src/storage.rs

# Fix unused variable
echo "Fixing unused variables..."
sed -i '' 's/let state_clone = shared_state.clone();/let _state_clone = shared_state.clone();/' src/main.rs

# Fix deprecated methods
echo "Fixing deprecated ndarray methods..."
sed -i '' 's/\.into_raw_vec()/\.into_raw_vec_and_offset().0/' src/ml.rs

# Add allow(dead_code) attributes for unused functions that might be used later
echo "Adding allow attributes for intentionally unused code..."

# For types.rs - add at the top of the file
sed -i '' '1s/^/#![allow(dead_code)]\n/' src/types.rs 2>/dev/null || true

# For chains.rs - add allow attribute before the impl block
sed -i '' '/impl ChainManager {/i\
#[allow(dead_code)]' src/chains.rs

# For dexs.rs - add allow attribute
sed -i '' '/impl DexManager {/i\
#[allow(dead_code)]' src/dexs.rs

# For ml.rs - add allow attribute
sed -i '' '/impl MLAnalyzer {/i\
#[allow(dead_code)]' src/ml.rs

# For storage.rs - add allow attribute
sed -i '' '/impl StorageEngine {/i\
#[allow(dead_code)]' src/storage.rs

# For metrics.rs - add allow attribute
sed -i '' '/impl MetricsServer {/i\
#[allow(dead_code)]' src/metrics.rs

# Fix the rust-objcopy warning by setting strip to none in Cargo.toml
echo "Fixing strip configuration..."
if ! grep -q '\[profile.release\]' Cargo.toml; then
    echo "" >> Cargo.toml
    echo "[profile.release]" >> Cargo.toml
    echo "strip = false" >> Cargo.toml
else
    # Check if strip is already set
    if ! grep -q 'strip' Cargo.toml; then
        sed -i '' '/\[profile.release\]/a\
strip = false' Cargo.toml
    fi
fi

echo "Building with fixes..."
cargo build --release

if [ $? -eq 0 ]; then
    echo ""
    echo "SUCCESS! Project compiled with warnings fixed!"
    echo ""
    echo "You can now run your crypto arbitrage scanner:"
    echo "  cargo run --release"
    echo ""
    echo "The application will:"
    echo "  - Monitor multiple chains for arbitrage opportunities"
    echo "  - Store opportunities in RocksDB"
    echo "  - Expose metrics on port 9090"
    echo "  - Run a WebSocket server on port 8080"
else
    echo "Build failed. Check the output above."
fi