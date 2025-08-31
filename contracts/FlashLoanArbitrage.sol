// SPDX-License-Identifier: MIT
pragma solidity ^0.8.19;

import "@aave/core-v3/contracts/flashloan/base/FlashLoanSimpleReceiverBase.sol";
import "@aave/core-v3/contracts/interfaces/IPoolAddressesProvider.sol";
import "@uniswap/v2-periphery/contracts/interfaces/IUniswapV2Router02.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract FlashLoanArbitrage is FlashLoanSimpleReceiverBase {
    address private immutable owner;
    uint256 private constant MAX_SLIPPAGE = 300; // 3%
    
    // DEX Routers
    IUniswapV2Router02 private constant UNISWAP_V2 = 
        IUniswapV2Router02(0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D);
    IUniswapV2Router02 private constant SUSHISWAP = 
        IUniswapV2Router02(0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F);
    IUniswapV2Router02 private constant SHIBASWAP = 
        IUniswapV2Router02(0x03f7724180AA6b939894B5Ca4314783B0b36b329);
    
    mapping(address => bool) public authorizedCallers;
    
    event ArbitrageExecuted(
        address indexed asset,
        uint256 amountBorrowed,
        uint256 profit,
        address buyDex,
        address sellDex
    );
    
    event FlashLoanInitiated(
        address indexed asset,
        uint256 amount
    );
    
    modifier onlyOwner() {
        require(msg.sender == owner, "Not owner");
        _;
    }
    
    modifier onlyAuthorized() {
        require(msg.sender == owner || authorizedCallers[msg.sender], "Not authorized");
        _;
    }
    
    constructor(address _addressProvider) 
        FlashLoanSimpleReceiverBase(IPoolAddressesProvider(_addressProvider)) 
    {
        owner = msg.sender;
    }
    
    function executeArbitrage(
        address asset,
        uint256 amount,
        address buyRouter,
        address sellRouter,
        address[] calldata buyPath,
        address[] calldata sellPath,
        uint256 minProfit
    ) external onlyAuthorized {
        bytes memory params = abi.encode(
            buyRouter,
            sellRouter,
            buyPath,
            sellPath,
            minProfit
        );
        
        emit FlashLoanInitiated(asset, amount);
        
        POOL.flashLoanSimple(
            address(this),
            asset,
            amount,
            params,
            0
        );
    }
    
    function executeOperation(
        address asset,
        uint256 amount,
        uint256 premium,
        address initiator,
        bytes calldata params
    ) external override returns (bool) {
        require(msg.sender == address(POOL), "Invalid caller");
        require(initiator == address(this), "Invalid initiator");
        
        (
            address buyRouter,
            address sellRouter,
            address[] memory buyPath,
            address[] memory sellPath,
            uint256 minProfit
        ) = abi.decode(params, (address, address, address[], address[], uint256));
        
        uint256 amountOwed = amount + premium;
        
        // Execute arbitrage
        IERC20(asset).approve(buyRouter, amount);
        
        uint256[] memory amounts = IUniswapV2Router02(buyRouter).swapExactTokensForTokens(
            amount,
            0,
            buyPath,
            address(this),
            block.timestamp
        );
        
        uint256 outputAmount = amounts[amounts.length - 1];
        address outputToken = buyPath[buyPath.length - 1];
        
        IERC20(outputToken).approve(sellRouter, outputAmount);
        
        uint256[] memory sellAmounts = IUniswapV2Router02(sellRouter).swapExactTokensForTokens(
            outputAmount,
            amountOwed,
            sellPath,
            address(this),
            block.timestamp
        );
        
        uint256 finalAmount = sellAmounts[sellAmounts.length - 1];
        require(finalAmount >= amountOwed + minProfit, "Insufficient profit");
        
        uint256 profit = finalAmount - amountOwed;
        
        emit ArbitrageExecuted(
            asset,
            amount,
            profit,
            buyRouter,
            sellRouter
        );
        
        // Approve flash loan repayment
        IERC20(asset).approve(address(POOL), amountOwed);
        
        return true;
    }
    
    function setAuthorizedCaller(address caller, bool authorized) external onlyOwner {
        authorizedCallers[caller] = authorized;
    }
    
    function withdrawToken(address token) external onlyOwner {
        uint256 balance = IERC20(token).balanceOf(address(this));
        require(balance > 0, "No balance");
        IERC20(token).transfer(owner, balance);
    }
    
    function withdrawETH() external onlyOwner {
        uint256 balance = address(this).balance;
        require(balance > 0, "No ETH balance");
        (bool success, ) = owner.call{value: balance}("");
        require(success, "ETH transfer failed");
    }
    
    receive() external payable {}
}