use ethers::prelude::*;
use ethers::signers::{LocalWallet, Signer};

fn main() {
    println!("🔑 Generating new wallet for arbitrage bot...\n");
    
    let wallet = LocalWallet::new(&mut rand::thread_rng());
    let address = format!("{:?}", wallet.address());
    let private_key = hex::encode(wallet.signer().to_bytes());
    
    println!("=================================");
    println!("WALLET GENERATED SUCCESSFULLY");
    println!("=================================");
    println!("Address: {}", address);
    println!("Private Key: {}", private_key);
    println!("=================================");
    
    println!("\n📝 Add to .env file:");
    println!("PRIVATE_KEY={}", private_key);
    
    println!("\n⚠️  SECURITY NOTES:");
    println!("1. Never share or commit your private key");
    println!("2. Send only small amounts of ETH for gas");
    println!("3. Save this information securely");
    
    println!("\n💰 Fund this address on Arbitrum:");
    println!("https://arbiscan.io/address/{}", address);
}
