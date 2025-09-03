use ethers::prelude::*;

pub struct FlashLoanProvider {
    balancer_vault: Address,
    aave_pool: Address,
}

impl FlashLoanProvider {
    pub fn new() -> Self {
        Self {
            // Arbitrum addresses
            balancer_vault: "0xBA12222222228d8Ba445958a75a0704d566BF2C8"
                .parse::<Address>().unwrap(),
            aave_pool: "0x794a61358D6845594F94dc1DB02A252b5b4814aD"
                .parse::<Address>().unwrap(),
        }
    }
    
    pub fn get_flashloan_abi() -> ethers::abi::Abi {
        ethers::abi::parse_abi(&[
            "function flashLoan(address recipient, address[] tokens, uint256[] amounts, bytes userData)",
            "function receiveFlashLoan(address[] tokens, uint256[] amounts, uint256[] feeAmounts, bytes userData)",
        ]).unwrap()
    }
}