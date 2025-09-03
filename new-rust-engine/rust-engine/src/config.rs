// rust-engine/src/config.rs
use ethers::prelude::*;

#[derive(Clone)]
pub struct ChainConfig {
    pub name: String,
    pub rpc: String,
    pub chain_id: u64,
    pub flash_loan_providers: Vec<Address>,
    pub dexes: Vec<DexConfig>,
}

#[derive(Clone)]
pub struct DexConfig {
    pub name: String,
    pub factory: Address,
    pub router: Address,
    pub fee_bps: u16,
}