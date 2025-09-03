# Arbitrage Bot - Production Ready

## Quick Start

1. **Generate Wallet**
```bash
cargo run --example generate_wallet
```

2. **Configure Environment**
```bash
cp .env.example .env
# Edit .env with your private key and RPC URLs
```

3. **Deploy Smart Contract** (optional but recommended)
```bash
cd contracts
npx hardhat run scripts/deploy_contract.js --network arbitrum
```

4. **Fund Wallet**
- Send 0.05-0.1 ETH to your bot wallet for gas fees
- The bot uses flashloans, so no trading capital needed

5. **Start Bot**
```bash
./start_bot.sh
```

6. **Monitor**
```bash
./scripts/monitor.sh
```

## Safety Features

- ✅ Flashloans (no capital risk)
- ✅ Simulation before execution
- ✅ Gas price limits
- ✅ Minimum profit thresholds
- ✅ Emergency stop functionality

## Performance Optimizations

- WebSocket connections for real-time data
- Parallel price fetching
- Optimized Rust binary
- Smart contract for gas efficiency

## Security

- Never commit .env file
- Use hardware wallet for production
- Implement IP whitelist on RPC
- Monitor for unusual activity
- Set up alerts for large profits/losses

## Monitoring

Check bot status:
```bash
./scripts/monitor.sh
```

View logs:
```bash
tail -f logs/bot.log
```

## Disclaimer

This bot is for educational purposes. Real arbitrage is highly competitive.
You may lose money due to:
- Gas fees
- Frontrunning
- Slippage
- Smart contract bugs

Always test on testnet first!
