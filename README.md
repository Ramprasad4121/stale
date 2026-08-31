# stale

[![CI](https://img.shields.io/github/actions/workflow/status/Ramprasad4121/stale/test.yml?branch=main&label=CI)](https://github.com/Ramprasad4121/stale/actions) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Node](https://img.shields.io/badge/node-%3E%3D18-brightgreen)](https://nodejs.org) [![Tests](https://img.shields.io/badge/tests-19%20pass-brightgreen)](https://github.com/Ramprasad4121/stale/actions) [![stale](https://img.shields.io/badge/stale-guardrail-blue)](https://github.com/Ramprasad4121/stale)

Guardrail for onchain agents. Before an agent acts on a Chainlink price, `stale` checks whether that price is still fresh. If it is stale, expired, or not-yet-valid, it returns `BLOCK` and a notify message. It never sends a transaction.

This repo exists because an agent that trades, gates, or triggers on an old price can lose money. The fix is small and specific: read the official on-chain timestamp that comes with the price, compare it to the agent's freshness policy, and block when the data is not current. `stale` keeps that check in one place so every agent can reuse it.

For teams building agents on Chainlink — and for the CRE workflow that will run the same check inside the DON.

## Table of Contents

- [Mental model](#mental-model)
- [Layout — why it is this way](#layout--why-it-is-this-way)
- [Install](#install)
- [Usage — CLI](#usage--cli)
- [Library](#library)
- [Agent skill](#agent-skill)
- [Testing](#testing)
- [CRE simulation](#cre-simulation)
- [What this repo does not do](#what-this-repo-does-not-do)
- [How to Contribute](#how-to-contribute)
- [Credits](#credits)
- [License](#license)

## Mental model

```
age = now - updatedAt
if age < 0 or age > maxAge → BLOCK (fail closed)
else → ALLOW
```

`updatedAt` comes from `PriceFeedAggregator.latestRoundData()` and `decimals()` is fetched on-chain via `decimals()` — never hardcoded — source: [Data Feeds API Reference](https://docs.chain.link/data-feeds/api-reference) and [Price Feed Addresses (Ethereum Mainnet)](https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet).

- **Fail closed:** missing, unparseable, `updatedAt == 0`, `answer <= 0`, future timestamp, or RPC failure → `BLOCK`.
- **One feed for v1:** `ETH/USD` on `ethereum-mainnet` at `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419`. Other feeds and Data Streams (`validFromTimestamp` / `expiresAt`) are later.

## Layout — why it is this way

- `src/isStale.ts` — pure function, no RPC. The only place that decides `ALLOW` vs `BLOCK`. Keep it pure so it can be unit-tested and reused in both the CLI and the CRE WASM build.
- `src/quote.ts` — price math from `answer` + `decimals` via `viem` `formatUnits`. Quote is separate from freshness so either can be reused alone.
- `src/check.ts` — composition layer: `viem` `latestRoundData` + `decimals` → `isStale` → `quoteFromFeed` → JSON with `allowExecute`. No wallet, no write — the agent decides.
- `src/cli.ts` — thin wrapper around `checkPrice` for humans and CI. It adds flag parsing (`--rpc`, `--maxAge`, `--feed`, `--amount`, `--json`), truncates notify errors to 200 chars, and maps `ALLOW`/`BLOCK` to exit codes. If you add a new flag, add it here and pass it through to `checkPrice`.
- `cre/` — a separate CRE project (official TypeScript SDK + CLI). It reuses the same two pure rules from `cre/lib/` (vendored copies of `src/isStale.ts` / `src/quote.ts` with `keep in sync` headers — the CRE WASM build cannot import `../../src`). Do not add CRE config to `src/`.

If you need to support a new feed, chain, or Streams timestamp, start in `src/` and keep `cre/lib/` in sync. Tests live next to the code they cover.

## Install

```bash
npm install
```

Requires `node >= 18` (`@types/node 22.7.0`, `tsx 4.19+`, `typescript 5.6+`). ESM only (`type: module`).

## Usage — CLI

`maxAge` is required — there is no default. The agent must choose its policy.

```bash
# via tsx (dev) — publicnode is the default example (no private RPC needed for read-only)
npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 3600
npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5
npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5 --json
npx tsx src/cli.ts --rpc $RPC_URL --maxAge 60 --feed 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419

# after build
npm run build
node dist/cli.js --rpc <url> --maxAge <seconds>
node dist/cli.js --rpc <url> --maxAge <seconds> --amount 0.5 --json
```

- `--amount <eth>` — human ETH for `quoteUsd = amountEth * priceUsd`. Bad value → `BLOCK` exit 1.
- `--json` — single JSON object only. Default is human lines. Use `--json` when piping to an agent.
- `--feed` — override the default `ETH/USD` proxy (useful for testing). Address must be `0x` + 40 hex.

**Exit codes:** `0` = `ALLOW`, `1` = `BLOCK` (agents can gate on exit code).

**Human output (default):**
```
ALLOW — fresh: age 12s <= maxAge 3600s — ALLOW
feed=0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 answer=300012345678 updatedAt=1724520000 age=12s maxAge=3600s now=1724520012
priceUsd=3000.12345678 amountEth=0.5 quoteUsd=1500.06172839
execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}
```
```
BLOCK — stale: age 5400s > maxAge 3600s — BLOCK
feed=0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 answer=300012345678 updatedAt=1724514600 age=5400s maxAge=3600s now=1724520000
priceUsd=3000.12345678 amountEth=null quoteUsd=null
notify: price is stale or not-yet-valid — do not act
execute: skipped
```

**JSON output (`--json`):**
```json
{"decision":"ALLOW","reason":"fresh: age 12s <= maxAge 3600s — ALLOW","feed":"0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419","answer":"300012345678","priceUsd":3000.12345678,"amountEth":0.5,"quoteUsd":1500.06172839,"updatedAt":"1724520000","ageSeconds":12,"maxAgeSeconds":3600,"now":1724520012,"allowExecute":true}
```
`priceUsd` is `Number(formatUnits(answer, decimals))`; `quoteUsd` is `amountEth * priceUsd` or `null`. `allowExecute` is true only when `decision === "ALLOW"`. On `BLOCK`, `execute: skipped`; on `ALLOW`, `execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}` — no Aave/CCIP/CRE-write/Agents call, no wallet, no broadcast. Notify errors are truncated to 200 chars.

## Library

```ts
import { isStale, quoteFromFeed, checkPrice } from "./src/index.js";

// pure, no RPC — reuse in tests, CRE, or your own guard
isStale({ updatedAt: 1724520000n, nowSeconds: Math.floor(Date.now()/1000), maxAgeSeconds: 60 });

// price math — no RPC
quoteFromFeed({ answer: 245377000000n, decimals: 8, amountEth: 0.5 });
// → { priceUsd: 2453.77, quoteUsd: 1226.885 }

// full check — viem + isStale + quote, no wallet, no tx, fail closed
const r = await checkPrice({
  rpc: "https://ethereum-rpc.publicnode.com",
  feed: "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419",
  maxAgeSeconds: 60,
  amountEth: 0.5,
});
// r has same shape as --json
```

`src/index.ts` is the public surface. If you add a new pure rule, export it from there so both the CLI and `cre` can import it.

## Agent skill

`skill.md` at the repo root is the copy-paste interface for agents. It shows the three imports above plus the CLI, with no keys or live calls. Keep it in sync with `src/index.ts`.

## Testing

Go the extra mile — tests are the confidence that the guard will not allow a stale price:

```bash
npm test          # tsx --test src/*.test.ts — 19 tests, no live RPC
```

Three focused suites, no live RPC (mocks `viem` `readContract`):

- `isStale` — fresh, stale, future, missing, `maxAge 0`, invalid `now`/`maxAge`, `bigint` including hex.
- `quote` — `2453.77`, `*0.5 → 1226.885`, `null` amount, `0n`, bad decimals, `NaN` amount.
- `checkPrice` — fresh `ALLOW`, stale `BLOCK`, `updatedAt 0` `BLOCK`, `answer <= 0` `BLOCK`, `decimals` throw `BLOCK`.

Add new cases in `src/*.test.ts` next to the code. Keep `npm test` green — CI runs `node 22`, `npm ci`, `npm test` in `.github/workflows/test.yml`.

## CRE simulation

The same check runs as a CRE TypeScript workflow in `cre/` (official SDK, `cron` trigger as `read-data-feeds-ts` template). It reads the same `0x5f4e…` feed via `PriceFeedAggregator` `latestRoundData` + `decimals`, then `isStale` + `quote`. See `cre/README.md` for details.

Staging `maxAgeSeconds` is `3600` so a local simulate can return `ALLOW` (on-chain age is ~2800s); production stays `60`. The workflow is chain-read only — no EVM write, no wallet, no `--broadcast`.

```bash
cd cre
cre workflow simulate workflows/stale \
  --target staging-settings \
  --non-interactive \
  --trigger-index 0 \
  --allow-insecure-rpc
```

Note: without `--non-interactive` the CLI can hang. `429` from the RPC → `BLOCK` (fail closed). No private key, no DON deploy.

## What this repo does not do

- No transactions, swaps, bridges, or `supply` — it only returns `ALLOW`/`BLOCK`.
- No `x402`, `CCIP`, or `Agents` execution — those are later.
- Single feed, single chain (`ethereum-mainnet` `ETH/USD`). Multi-feed and Data Streams (`validFromTimestamp`/`expiresAt`) are next.

## How to Contribute

`stale` is open source — your help is welcome:

- **PRs only, not direct to `main`** — `opencode-review` runs on PRs (`opened`, `synchronize`, `reopened`, `ready_for_review`). Push to a branch, open a PR against `main`.
- **Keep tests green** — `npm test` must stay `19 pass`. No live RPC in tests (mock `readContract`).
- **No private keys or local lore** — never commit `.env`, private keys, `AGENTS.md` or `LINUS.md` (both are `.gitignore` local-only).
- **Official fields only, fail closed** — use `latestRoundData.updatedAt` + `decimals()`, don’t hardcode decimals, `updatedAt == 0` / `answer <= 0` / stale / future → `BLOCK`.
- **CRE:** run the simulate command above to verify; never `cre workflow deploy` without a human saying so.

See `CONTRIBUTING.md` for the full guide. By contributing you agree that your contributions will be under the same `MIT` license.

## Credits

Built by [Ramprasad Edigi](https://ramprasadgoud.dev) — smart contract security researcher and Chainlink Community Advocate. The project started like many — a hurried `README` with no structure — and grew into a focused guardrail through practice, continuous learning, and studying how good projects document the *why* behind their layout.

Thanks to:
- **Chainlink** — [Data Feeds](https://docs.chain.link/data-feeds) and [CRE](https://docs.chain.link/cre) docs and the `read-data-feeds-ts` template.
- **viem**, **zod**, **TypeScript**, and `node:test` — for the type-safe, well-documented foundations.
- **Contributors** — you? See `CONTRIBUTING.md` to get started. Also thanks to the builders whose `README`s taught the value of a good first file — your `README` is the first thing a visitor sees, and it helps them decide to stay, learn, and contribute.

If this helped, share it — and share your newly crafted `README` with [@0xramprasad](https://x.com/0xramprasad).

## License

MIT — see [LICENSE](LICENSE). You may use this in commercial projects. If you need help choosing a license, see [choosealicense.com](https://choosealicense.com/).
