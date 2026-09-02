# 🛡️ stale

> The ultimate, un-bypassable Linux-level security standard for autonomous AI agents that touch money.

`stale` is a pure, fast, dependency-light TypeScript library that provides an impenetrable 10-point defense system for AI agents transacting on-chain. It strictly enforces least-privilege execution and fails closed on ANY anomaly.

[![npm version](https://img.shields.io/npm/v/@ramprasad4121/stale.svg)](https://www.npmjs.com/package/@ramprasad4121/stale)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Why `stale`?

Autonomous agents are often naive to the dark forest of DeFi. They blindly execute trades with infinite slippage, route transactions through public mempools, and approve infinite tokens to vulnerable routers.

`stale` intercepts intent. Before your agent signs a transaction, it MUST pass through the `stale` guardrails.

## Features (The 10-Point Defense System)

1. **Stale/Corrupted Pricing**: Queries Chainlink Data Feeds to block execution if the oracle price is older than your safety threshold.
2. **L2 Sequencer Outages**: Natively checks Arbitrum, Optimism, Base, Scroll, zkSync, and Metis sequencer liveness. Blocks execution if the L2 sequencer is down or in the 1-hour grace period.
3. **Illiquid Honeypots**: Evaluates Uniswap V2/V3 liquidity depth. Blocks agents from trading into shallow pools.
4. **MEV Sandwich Attacks**: Mathematically computes exact `minAmountOut` slippage boundaries based on Chainlink feed prices, making MEV bleeding impossible.
5. **Malicious Infinite Approvals**: Blocks agents from attempting `approve(MaxUint256)`, enforcing strict Exact Amount Approvals.
6. **Network Gas Spikes**: Circuit breaker that halts execution if network base fees spike during gas wars, preserving the agent's treasury.
7. **Insolvency RPC Spam**: Checks if the agent holds sufficient Native/ERC20 balances _before_ attempting a trade, preventing them from looping failed transactions.
8. **Protocol Emergency Pauses**: Actively queries target protocols (like USDC or Aave) for `paused()` state. Blocks interaction if the multi-sig has engaged the emergency pause.
9. **Advanced Transaction Simulation**: Runs native `eth_call` to trace the agent's transaction against the current block. If the simulation reverts for _any_ reason, execution is blocked.
10. **OFAC Compliance & EOA Phishing**: Queries Chainlink's OFAC Oracle to prevent interaction with sanctioned entities, and verifies bytecode to prevent agents from sending funds to EOA phishing scammers.

## Installation

```bash
npm install @ramprasad4121/stale
# or
yarn add @ramprasad4121/stale
# or
pnpm add @ramprasad4121/stale
```

## Quick Start

```typescript
import { checkPrice, checkApproval, simulateTx } from "@ramprasad4121/stale";

// 1. Ensure the Chainlink Oracle price is fresh (max 1 hour old)
const priceGuard = await checkPrice({
  rpc: "https://ethereum-rpc.publicnode.com",
  feed: "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419", // ETH/USD
  maxAge: 3600,
});
if (priceGuard.decision === "BLOCK") throw new Error(priceGuard.reason);

// 2. Ensure the agent isn't attempting an infinite approval
const approvalGuard = checkApproval({
  token: "0xA0b8...",
  spender: "0x68b3...",
  amount: 1000000n, // exact amount
});
if (approvalGuard.decision === "BLOCK") throw new Error(approvalGuard.reason);

// 3. Simulate the final transaction against the latest block
const simGuard = await simulateTx({
  rpc: "https://ethereum-rpc.publicnode.com",
  account: "0xAgentWallet",
  to: "0xTargetProtocol",
  data: "0xTransactionData",
});
if (simGuard.decision === "BLOCK") throw new Error(simGuard.reason);

// => Agent is safe to execute the transaction
```

## MCP Server Integration

`stale` includes a built-in [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server, allowing AI assistants (like Claude) to directly validate their intents before executing on-chain.

```bash
npx @ramprasad4121/stale
```

## License

MIT © Ramprasad4121
