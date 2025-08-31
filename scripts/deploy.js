const { ethers } = require("hardhat");
const fs = require("fs");
require("dotenv").config();

async function main() {
  console.log("Deploying Flash Loan Arbitrage Contract...");
  
  const [deployer] = await ethers.getSigners();
  console.log("Deploying with account:", deployer.address);
  
  const balance = await deployer.getBalance();
  console.log("Account balance:", ethers.utils.formatEther(balance), "ETH");
  
  // Aave V3 Pool Address Provider
  const AAVE_ADDRESS_PROVIDER = "0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e";
  
  // Deploy contract
  const FlashLoanArbitrage = await ethers.getContractFactory("FlashLoanArbitrage");
  const flashLoanArbitrage = await FlashLoanArbitrage.deploy(AAVE_ADDRESS_PROVIDER);
  await flashLoanArbitrage.deployed();
  
  console.log("✅ FlashLoanArbitrage deployed to:", flashLoanArbitrage.address);
  
  // Skip confirmations on local network
  if (network.name !== "localhost" && network.name !== "hardhat") {
    console.log("Waiting for block confirmations...");
    await flashLoanArbitrage.deployTransaction.wait(6);
  }
  
  // Save deployment info
  const deploymentInfo = {
    contractAddress: flashLoanArbitrage.address,
    deployer: deployer.address,
    network: network.name,
    timestamp: new Date().toISOString()
  };
  
  fs.writeFileSync("./deployment.json", JSON.stringify(deploymentInfo, null, 2));
  
  console.log("\n📁 Deployment saved to deployment.json");
  console.log("\nNext steps:");
  console.log(`1. Update .env: CONTRACT_ADDRESS=${flashLoanArbitrage.address}`);
  console.log("2. Run: npm run test:local");
}

main()
  .then(() => process.exit(0))
  .catch((error) => {
    console.error(error);
    process.exit(1);
  });