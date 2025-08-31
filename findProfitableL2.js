const { ethers } = require("ethers");

// Arbitrum - where gas is cheap and profits are possible
const provider = new ethers.providers.JsonRpcProvider("https://arb1.arbitrum.io/rpc");

async function scan() {
  const gasPrice = await provider.getGasPrice();
  console.log("Arbitrum gas:", ethers.utils.formatUnits(gasPrice, "gwei"), "gwei (100x cheaper than mainnet!)");
  
  // Your arbitrage logic here
  // Gas costs $0.05 instead of $50
}

scan();