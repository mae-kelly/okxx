const { ethers } = require("ethers");
require("dotenv").config();

const provider = new ethers.providers.JsonRpcProvider(
  `https://eth-mainnet.g.alchemy.com/v2/${process.env.ALCHEMY_API_KEY}`
);

const PAIRS = [
  { from: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", to: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", name: "USDC→WETH", decimals: 6 },
  { from: "0xdAC17F958D2ee523a2206206994597C13D831ec7", to: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", name: "USDT→WETH", decimals: 6 },
  { from: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", to: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", name: "WETH→USDC", decimals: 18 }
];

const uni = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
const sushi = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
const abi = ["function getAmountsOut(uint,address[]) view returns(uint[])"];

async function scan() {
  console.clear();
  const block = await provider.getBlockNumber();
  console.log(`Block: ${block} | Time: ${new Date().toLocaleTimeString()}\n`);
  
  for (const pair of PAIRS) {
    try {
      const uniRouter = new ethers.Contract(uni, abi, provider);
      const sushiRouter = new ethers.Contract(sushi, abi, provider);
      
      const amount = ethers.utils.parseUnits("10000", pair.decimals);
      
      const [uniOut, sushiOut] = await Promise.all([
        uniRouter.getAmountsOut(amount, [pair.from, pair.to]),
        sushiRouter.getAmountsOut(amount, [pair.from, pair.to])
      ]);
      
      const diff = uniOut[1].sub(sushiOut[1]).abs();
      const spread = diff.mul(10000).div(uniOut[1]).toNumber() / 100;
      
      const profitable = spread > 0.6 ? "✅ PROFITABLE" : spread > 0.5 ? "⚠️  MARGINAL" : "❌ UNPROFITABLE";
      
      console.log(`${pair.name}: ${spread.toFixed(3)}% spread | ${profitable}`);
    } catch(e) {
      console.log(`${pair.name}: Error`);
    }
  }
}

setInterval(scan, 2000);
scan();