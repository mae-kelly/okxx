// File: src/main.rs

mod l2_scanner;
mod l2_executor;

use l2_scanner::L2ArbitrageScanner;
use l2_executor::L2ExecutionEngine;
use std::env;
use tokio::task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════════╗");
    println!("║   L2 MULTI-CHAIN ARBITRAGE BOT v1.0     ║");
    println!("║   Scanning ALL pairs across L2 networks  ║");
    println!("╚══════════════════════════════════════════╝\n");
    
    // Get private key from environment variable for security
    let private_key = env::var("PRIVATE_KEY")
        .unwrap_or_else(|_| "0x0000000000000000000000000000000000000000000000000000000000000001".to_string());
    
    // Initialize components
    let scanner = L2ArbitrageScanner::new().await?;
    let executor = L2ExecutionEngine::new(&private_key).await?;
    
    // Check wallet balances
    executor.check_balances().await?;
    
    // Discover all trading pairs
    println!("📊 Phase 1: Discovering all trading pairs...\n");
    scanner.discover_all_pairs().await?;
    
    // Start scanning in background
    let scanner_handle = task::spawn(async move {
        if let Err(e) = scanner.scan_opportunities().await {
            eprintln!("Scanner error: {}", e);
        }
    });
    
    // Wait for scanner
    scanner_handle.await?;
    
    Ok(())
}