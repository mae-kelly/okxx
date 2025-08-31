const { ethers } = require("ethers");

async function scan() {
  const provider = new ethers.providers.JsonRpcProvider("http://localhost:8545");
  
  const uni = "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D";
  const sushi = "0xd9e1cE17f2641f24aE83637ab66a2cca9C378B9F";
  const WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
  const USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
  
  const abi = ["function getAmountsOut(uint,address[]) view returns(uint[])"];
  
  const uniRouter = new ethers.Contract(uni, abi, provider);
  const sushiRouter = new ethers.Contract(sushi, abi, provider);
  
  const amount = ethers.utils.parseUnits("10000", 6);
  
  try {
    const [uniOut] = await Promise.all([
      uniRouter.getAmountsOut(amount, [USDC, WETH]),
      sushiRouter.getAmountsOut(amount, [USDC, WETH])
    ]).then(([u, s]) => [u[1], s[1]]);
    
    const sushiOut = (await sushiRouter.getAmountsOut(amount, [USDC, WETH]))[1];
    
    const diff = uniOut.sub(sushiOut).abs();
    const spread = diff.mul(10000).div(uniOut).toNumber() / 100;
    
    console.log(`USDC→WETH Spread: ${spread.toFixed(3)}% | Profit on $10k: $${(spread * 100).toFixed(2)}`);
  } catch(e) {
    console.log("Error:", e.message);
  }
}

setInterval(scan, 1000);
scan();