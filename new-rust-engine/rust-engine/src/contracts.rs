// rust-engine/src/contracts.rs
use ethers::prelude::*;
use ethers::abi::Abi;
use anyhow::Result;

// Contract ABIs
pub fn get_flash_loan_abi() -> Result<Abi> {
    Ok(ethers::abi::parse_abi(&[
        "function flashLoan(address receiver, address[] tokens, uint256[] amounts, bytes userData)",
        "function flashLoanSimple(address receiver, address asset, uint256 amount, bytes params, uint16 referralCode)",
        "function receiveFlashLoan(address[] tokens, uint256[] amounts, uint256[] fees, bytes userData)",
    ])?)
}

pub fn get_dex_router_abi() -> Result<Abi> {
    Ok(ethers::abi::parse_abi(&[
        "function swapExactTokensForTokens(uint amountIn, uint amountOutMin, address[] path, address to, uint deadline) returns (uint[] amounts)",
        "function swapTokensForExactTokens(uint amountOut, uint amountInMax, address[] path, address to, uint deadline) returns (uint[] amounts)",
        "function getAmountsOut(uint amountIn, address[] path) view returns (uint[] amounts)",
        "function getAmountsIn(uint amountOut, address[] path) view returns (uint[] amounts)",
    ])?)
}

pub fn get_pair_abi() -> Result<Abi> {
    Ok(ethers::abi::parse_abi(&[
        "function getReserves() view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
        "function token0() view returns (address)",
        "function token1() view returns (address)",
        "function swap(uint amount0Out, uint amount1Out, address to, bytes data)",
        "function sync()",
        "function skim(address to)",
    ])?)
}

pub fn get_factory_abi() -> Result<Abi> {
    Ok(ethers::abi::parse_abi(&[
        "function getPair(address tokenA, address tokenB) view returns (address pair)",
        "function allPairs(uint) view returns (address)",
        "function allPairsLength() view returns (uint)",
        "function createPair(address tokenA, address tokenB) returns (address pair)",
    ])?)
}

pub fn get_erc20_abi() -> Result<Abi> {
    Ok(ethers::abi::parse_abi(&[
        "function balanceOf(address) view returns (uint256)",
        "function transfer(address to, uint256 amount) returns (bool)",
        "function approve(address spender, uint256 amount) returns (bool)",
        "function allowance(address owner, address spender) view returns (uint256)",
        "function decimals() view returns (uint8)",
        "function symbol() view returns (string)",
    ])?)
}

// Optimized Arbitrage Contract Bytecode (simplified example)
pub fn get_arbitrage_bytecode() -> Bytes {
    // This would be your actual compiled arbitrage contract
    // It should handle flash loans and execute swaps atomically
    Bytes::from(hex::decode(
        "608060405234801561001057600080fd5b50"
    ).unwrap_or_default())
}

// Contract addresses for different chains
pub struct ChainContracts {
    pub chain_id: u64,
    pub flash_loan_providers: Vec<FlashProvider>,
    pub dex_factories: Vec<DexFactory>,
}

#[derive(Clone, Debug)]
pub struct FlashProvider {
    pub name: String,
    pub address: Address,
    pub fee_bps: u16, // basis points
}

#[derive(Clone, Debug)]
pub struct DexFactory {
    pub name: String,
    pub factory: Address,
    pub router: Address,
    pub fee_bps: u16,
}

pub fn get_chain_contracts(chain_id: u64) -> ChainContracts {
    match chain_id {
        42161 => ChainContracts { // Arbitrum
            chain_id,
            flash_loan_providers: vec![
                FlashProvider {
                    name: "Aave V3".to_string(),
                    address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse().unwrap(),
                    fee_bps: 9, // 0.09%
                },
                FlashProvider {
                    name: "Balancer".to_string(),
                    address: "0xBA12222222228d8Ba445958a75a0704d566BF2C8".parse().unwrap(),
                    fee_bps: 5, // 0.05%
                },
            ],
            dex_factories: vec![
                DexFactory {
                    name: "Uniswap V3".to_string(),
                    factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap(),
                    router: "0xE592427A0AEce92De3Edee1F18E0157C05861564".parse().unwrap(),
                    fee_bps: 30,
                },
                DexFactory {
                    name: "Sushiswap".to_string(),
                    factory: "0xc35DADB65012eC5796536bD9864eD8773aBc74C4".parse().unwrap(),
                    router: "0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506".parse().unwrap(),
                    fee_bps: 30,
                },
                DexFactory {
                    name: "Camelot".to_string(),
                    factory: "0x6EcCab422D763aC031210895C81787E87B43A652".parse().unwrap(),
                    router: "0xc873fEcbd354f5A56E00E710B90EF4201db2448d".parse().unwrap(),
                    fee_bps: 30,
                },
                DexFactory {
                    name: "TraderJoe".to_string(),
                    factory: "0xaE4EC9901c3076D0DdBe76A520F9E90a6227aCB7".parse().unwrap(),
                    router: "0xb4315e873dBcf96Ffd0acd8EA43f689D8c20fB30".parse().unwrap(),
                    fee_bps: 30,
                },
            ],
        },
        10 => ChainContracts { // Optimism
            chain_id,
            flash_loan_providers: vec![
                FlashProvider {
                    name: "Aave V3".to_string(),
                    address: "0x794a61358D6845594F94dc1DB02A252b5b4814aD".parse().unwrap(),
                    fee_bps: 9,
                },
            ],
            dex_factories: vec![
                DexFactory {
                    name: "Velodrome V2".to_string(),
                    factory: "0xF1046053aa5682b4F9a81b5481394DA16BE5FF5a".parse().unwrap(),
                    router: "0xa062aE8A9c5e11aaA026fc2670B0D65cCc8B2858".parse().unwrap(),
                    fee_bps: 30,
                },
                DexFactory {
                    name: "Uniswap V3".to_string(),
                    factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984".parse().unwrap(),
                    router: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45".parse().unwrap(),
                    fee_bps: 30,
                },
            ],
        },
        8453 => ChainContracts { // Base
            chain_id,
            flash_loan_providers: vec![
                FlashProvider {
                    name: "Aave V3".to_string(),
                    address: "0xA238Dd80C259a72e81d7e4664a9801593F98d1c5".parse().unwrap(),
                    fee_bps: 9,
                },
            ],
            dex_factories: vec![
                DexFactory {
                    name: "Aerodrome".to_string(),
                    factory: "0x420DD381b31aEf6683db6B902084cB0FFECe40Da".parse().unwrap(),
                    router: "0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43".parse().unwrap(),
                    fee_bps: 30,
                },
                DexFactory {
                    name: "BaseSwap".to_string(),
                    factory: "0xFDa619b6d20975be80A10332cD39b9a4b0FAa8BB".parse().unwrap(),
                    router: "0x327Df1E6de05895d2ab08513aaDD9313Fe505d86".parse().unwrap(),
                    fee_bps: 25,
                },
                DexFactory {
                    name: "Uniswap V3".to_string(),
                    factory: "0x33128a8fC17869897dcE68Ed026d694621f6FDfD".parse().unwrap(),
                    router: "0x2626664c2603336E57B271c5C0b26F421741e481".parse().unwrap(),
                    fee_bps: 30,
                },
            ],
        },
        _ => ChainContracts {
            chain_id,
            flash_loan_providers: vec![],
            dex_factories: vec![],
        },
    }
}