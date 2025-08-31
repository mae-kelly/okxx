const { ethers } = require("ethers");
require("dotenv").config();

async function scanLive() {
  const provider = new ethers.providers.JsonRpcProvider(
    `https://eth-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
  );
  
  const uni = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
  const sushi = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
  const WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
  const USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
  
  const abi = ["function getAmountsOut(uint,address[]) view returns(uint[])"];
  
  const uniRouter = new ethers.Contract(uni, abi, provider);
  const sushiRouter = new ethers.Contract(sushi, abi, provider);
  
  console.log("Connected to LIVE Mainnet\n");
  
  async function check() {
    const amount = ethers.utils.parseUnits("10000", 6);
    
    try {
      const block = await provider.getBlockNumber();
      const [uniOut, sushiOut] = await Promise.all([
        uniRouter.getAmountsOut(amount, [USDC, WETH]),
        sushiRouter.getAmountsOut(amount, [USDC, WETH])
      ]);
      
      const diff = uniOut[1].sub(sushiOut[1]).abs();
      const spread = diff.mul(10000).div(uniOut[1]).toNumber() / 100;
      
      console.log(
        `Block ${block} | ` +
        `Spread: ${spread.toFixed(3)}% | ` +
        `Time: ${new Date().toLocaleTimeString()}`
      );
    } catch(e) {
      console.log("Error:", e.message);
    }
  }
  
  setInterval(check, 2000);
  check();
}

scanLive();