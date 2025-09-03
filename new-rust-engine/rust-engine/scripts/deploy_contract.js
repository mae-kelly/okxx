const { ethers } = require("hardhat");

async function main() {
    console.log("Deploying FlashLoanArbitrage contract...");
    
    const FlashLoanArbitrage = await ethers.getContractFactory("FlashLoanArbitrage");
    const contract = await FlashLoanArbitrage.deploy();
    await contract.deployed();
    
    console.log("Contract deployed to:", contract.address);
    console.log("Add this to your .env file:");
    console.log(`FLASHLOAN_CONTRACT=${contract.address}`);
}

main()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error(error);
        process.exit(1);
    });
