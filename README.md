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

- Fail closed: missing, unparseable, `updatedAt == 0`, or RPC failure → `BLOCK`.
- Official field only: `updatedAt` from `latestRoundData`. No invented APIs.

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
npx tsx src/cli.ts --rpc $RPC_URL --maxAge 60 --feed 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419

# after build
npm run build
node dist/cli.js --rpc <url> --maxAge <seconds>
```

**Exit codes:** `0` = ALLOW, `1` = BLOCK (so an agent can gate on it).

**Output examples:**

```
ALLOW — fresh: age 12s <= maxAge 3600s — ALLOW
feed=0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 answer=300012345678 updatedAt=1724520000 age=12s maxAge=3600s now=1724520012
```

```
BLOCK — stale: age 5400s > maxAge 3600s — BLOCK
feed=0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 answer=300012345678 updatedAt=1724514600 age=5400s maxAge=3600s now=1724520000
notify: price is stale or not-yet-valid — do not act
```

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
