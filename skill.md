# stale — agent skill

Guardrail for onchain agents. Call `checkPrice` before acting on a Chainlink price. Fail closed — stale, missing, or not-yet-valid → `BLOCK`.

No transaction is sent. No wallet. No Aave/CCIP/CRE/x402.

## When to use

Before any agent reads a Data Feed price to quote, gate, or trigger.

## How

```ts
import { checkPrice, isStale, quoteFromFeed } from "@ramprasad4121/stale";

// 1. Full check (viem + isStale + quote) — recommended
const r = await checkPrice({
  rpc: process.env.RPC_URL!,
  feed: "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419", // ETH/USD mainnet
  maxAgeSeconds: 60,
  amountEth: 0.5, // optional
});
 // r = { decision, reason, feed, answer, priceUsd, amountEth, quoteUsd, updatedAt, ageSeconds, maxAgeSeconds, now, allowExecute }
 // if !r.allowExecute → do not act, notify human: r.reason

// 2. Pure check (no RPC)
import { isStale } from "@ramprasad4121/stale";
isStale({ updatedAt: 1724520000n, nowSeconds: Math.floor(Date.now()/1000), maxAgeSeconds: 60 });
 // → { decision: "ALLOW"|"BLOCK", ageSeconds, reason }

// 3. Price math only
import { quoteFromFeed } from "@ramprasad4121/stale";
quoteFromFeed({ answer: 245377000000n, decimals: 8, amountEth: 0.5 });
// → { priceUsd: 2453.77, quoteUsd: 1226.885 }
```

## CLI

```bash
npx stale --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5
npx stale --rpc $RPC_URL --maxAge 60 --amount 0.5 --json
# exit 0 = ALLOW, 1 = BLOCK
# --json prints only JSON, default is human lines
```

Feed decimals are fetched on-chain via `decimals()` — not hardcoded. Default feed `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419` is Ethereum mainnet only — `checkPrice` verifies `eth_chainId === 1` (viem `getChainId`) and `BLOCK`s on mismatch. Source: https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet

## Contracts (JSON)

`checkPrice` returns same shape as `stale --json`:

```json
{"decision":"ALLOW","reason":"fresh: age 10s <= maxAge 60s — ALLOW","feed":"0x...","answer":"300000000000","priceUsd":3000,"amountEth":0.5,"quoteUsd":1500,"updatedAt":"1724519990","ageSeconds":10,"maxAgeSeconds":60,"now":1724520000,"allowExecute":true}
```

- `allowExecute` is `true` only when `decision === "ALLOW"` — it is permission, not an execution call. Do not rename the field.
- Dry-run: if `!allowExecute` → `execute: skipped`; if `true` → `execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}`. No transaction is sent.

## Out of scope

Aave, CCIP, CRE, x402, private keys, live Agents calls, multi-chain.
