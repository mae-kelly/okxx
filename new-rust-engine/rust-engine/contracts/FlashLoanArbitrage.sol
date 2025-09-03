// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

interface IERC20 {
    function transfer(address to, uint256 amount) external returns (bool);
    function approve(address spender, uint256 amount) external returns (bool);
    function balanceOf(address account) external view returns (uint256);
}

interface IFlashLoanRecipient {
    function receiveFlashLoan(
        IERC20[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external;
}

interface IBalancerVault {
    function flashLoan(
        IFlashLoanRecipient recipient,
        IERC20[] memory tokens,
        uint256[] memory amounts,
        bytes memory userData
    ) external;
}

interface IDEXRouter {
    function swapExactTokensForTokens(
        uint amountIn,
        uint amountOutMin,
        address[] calldata path,
        address to,
        uint deadline
    ) external returns (uint[] memory amounts);
}

contract FlashLoanArbitrage is IFlashLoanRecipient {
    address private owner;
    IBalancerVault private constant vault = IBalancerVault(0xBA12222222228d8Ba445958a75a0704d566BF2C8);
    
    // Arbitrum DEX routers
    address private constant UNISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    address private constant SUSHISWAP_ROUTER = 0x1b02dA8Cb0d097eB8D57A175b88c7D8b47997506;
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    constructor() {
        owner = msg.sender;
    }
    
    function executeFlashLoan(
        address token,
        uint256 amount,
        bytes calldata params
    ) external onlyOwner {
        IERC20[] memory tokens = new IERC20[](1);
        tokens[0] = IERC20(token);
        
        uint256[] memory amounts = new uint256[](1);
        amounts[0] = amount;
        
        vault.flashLoan(this, tokens, amounts, params);
    }
    
    function receiveFlashLoan(
        IERC20[] memory tokens,
        uint256[] memory amounts,
        uint256[] memory feeAmounts,
        bytes memory userData
    ) external override {
        require(msg.sender == address(vault), "Not vault");
        
        // Decode arbitrage parameters
        (address tokenA, address tokenB, bool buyFromUni) = abi.decode(
            userData, 
            (address, address, bool)
        );
        
        // Execute arbitrage
        _performArbitrage(
            address(tokens[0]), 
            tokenA, 
            tokenB, 
            amounts[0], 
            buyFromUni
        );
        
        // Repay flashloan
        tokens[0].transfer(address(vault), amounts[0] + feeAmounts[0]);
        
        // Send profit to owner
        uint256 profit = tokens[0].balanceOf(address(this));
        if (profit > 0) {
            tokens[0].transfer(owner, profit);
        }
    }
    
    function _performArbitrage(
        address flashToken,
        address tokenA,
        address tokenB,
        uint256 amount,
        bool buyFromUni
    ) private {
        address buyRouter = buyFromUni ? UNISWAP_ROUTER : SUSHISWAP_ROUTER;
        address sellRouter = buyFromUni ? SUSHISWAP_ROUTER : UNISWAP_ROUTER;
        
        // Approve routers
        IERC20(flashToken).approve(buyRouter, amount);
        
        // Buy on first DEX
        address[] memory path1 = new address[](2);
        path1[0] = tokenA;
        path1[1] = tokenB;
        
        uint[] memory amounts1 = IDEXRouter(buyRouter).swapExactTokensForTokens(
            amount,
            0,
            path1,
            address(this),
            block.timestamp + 300
        );
        
        // Approve for second swap
        IERC20(tokenB).approve(sellRouter, amounts1[1]);
        
        // Sell on second DEX
        address[] memory path2 = new address[](2);
        path2[0] = tokenB;
        path2[1] = tokenA;
        
        IDEXRouter(sellRouter).swapExactTokensForTokens(
            amounts1[1],
            0,
            path2,
            address(this),
            block.timestamp + 300
        );
    }
    
    function emergencyWithdraw(address token) external onlyOwner {
        if (token == address(0)) {
            payable(owner).transfer(address(this).balance);
        } else {
            IERC20(token).transfer(owner, IERC20(token).balanceOf(address(this)));
        }
    }
}
