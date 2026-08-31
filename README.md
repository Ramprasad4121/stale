# stale

Guardrail for onchain agents — checks an official Chainlink price timestamp and returns **ALLOW** or **BLOCK** + notify. Does not send a transaction.

For people building agents on Chainlink. v1 only reads one Data Feed.

## How it works

Reads `latestRoundData()` → `answer` + `updatedAt` from the official Data Feed proxy and compares age to `maxAgeSeconds`:

```
age = now - updatedAt
if age < 0 or age > maxAge → BLOCK (fail closed)
else → ALLOW
```

- Fail closed: missing, unparseable, `updatedAt == 0`, `answer <= 0`, or RPC failure → `BLOCK`.
- Official fields only: `updatedAt` from `latestRoundData` and `decimals()` for price scaling. No invented APIs. Decimals are fetched on-chain via `decimals()` — do not hardcode 8 — source: [docs.chain.link — Data Feeds API Reference](https://docs.chain.link/data-feeds/api-reference).

Feed for v1: **ETH/USD on Ethereum mainnet**
Proxy `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419` — source: [docs.chain.link — Price Feed Addresses (Ethereum Mainnet)](https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet)

## Install

```bash
npm install
```

## Usage

`maxAge` is required — no default. You must choose the policy.

```bash
# via tsx (dev)
npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 3600
npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5
npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5 --json
npx tsx src/cli.ts --rpc $RPC_URL --maxAge 60 --feed 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419

# after build
npm run build
node dist/cli.js --rpc <url> --maxAge <seconds>
node dist/cli.js --rpc <url> --maxAge <seconds> --amount 0.5 --json
```

- `--amount <eth>` — optional human ETH amount for quote (e.g. `0.5`). Bad value → `BLOCK` exit 1.
- `--json` — output single JSON object only. Default is human lines only.

**Exit codes:** `0` = ALLOW, `1` = BLOCK (so an agent can gate on it).

**Human output (default):**

```
ALLOW — fresh: age 12s <= maxAge 3600s — ALLOW
feed=0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 answer=300012345678 updatedAt=1724520000 age=12s maxAge=3600s now=1724520012
priceUsd=3000.12345678 amountEth=0.5 quoteUsd=1500.06172839 (decimals=8)
execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}
```

```
BLOCK — stale: age 5400s > maxAge 3600s — BLOCK
feed=0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 answer=300012345678 updatedAt=1724514600 age=5400s maxAge=3600s now=1724520000
priceUsd=3000.12345678 amountEth=null quoteUsd=null (decimals=8)
notify: price is stale or not-yet-valid — do not act
execute: skipped
```

**JSON output (`--json`):**

```json
{"decision":"ALLOW","reason":"fresh: age 12s <= maxAge 3600s — ALLOW","feed":"0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419","answer":"300012345678","priceUsd":3000.12345678,"amountEth":0.5,"quoteUsd":1500.06172839,"updatedAt":"1724520000","ageSeconds":12,"maxAgeSeconds":3600,"now":1724520012,"allowExecute":true}
```

Quote math: `priceUsd = Number(formatUnits(answer, decimals))` where `decimals` is read via `decimals()` on the feed; `quoteUsd = amountEth * priceUsd` if `--amount` given else `null`.

**Dry-run (v3):** if `allowExecute` is false prints `execute: skipped` and exits 1; if true prints `execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}` and exits 0. No Aave, CCIP, CRE, or Agents calls. No wallet or broadcast.

```bash
# programmatic
import { isStale } from "./src/isStale.js";
isStale({ updatedAt: 1724520000n, nowSeconds: Math.floor(Date.now()/1000), maxAgeSeconds: 3600 });
```

## What v1 does not do

- Does not send transactions, swaps, bridges, or supply.
- Does not use x402, CCIP, or Agents execution.
- Single chain / single feed (Ethereum mainnet ETH/USD). Multi-feed and Data Streams (`validFromTimestamp`/`expiresAt`) are later.

## License

MIT
