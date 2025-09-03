use ethers::prelude::*;
use ethers::signers::{LocalWallet, Signer};

fn main() {
    println!("🔑 Generating new wallet for arbitrage bot...\n");
    
    let wallet = LocalWallet::new(&mut rand::thread_rng());
    let address = wallet.address();
    let private_key = hex::encode(wallet.signer().to_bytes());
    
    println!("Address: {}", address);
    println!("Private Key: {}", private_key);
    
    println!("\n📝 Add to .env file:");
    println!("PRIVATE_KEY={}", private_key);
    
    println!("\n⚠️  IMPORTANT SECURITY NOTES:");
    println!("1. Never share or commit your private key");
    println!("2. Send only small amounts of ETH for gas (0.05-0.1 ETH)");
    println!("3. Use flashloans - don't hold large amounts in the wallet");
    println!("4. Consider using a hardware wallet for production");
    
    println!("\n💰 Fund this address with ETH on Arbitrum:");
    println!("https://arbiscan.io/address/{}", address);
}
