# stale — CRE simulation

Local simulation of the same check as `src/checkPrice` but via Chainlink CRE TypeScript workflow.

- Reads `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419` (ETH/USD Ethereum mainnet) via `PriceFeedAggregator` `latestRoundData` + `decimals` (chain-read only).
- Uses `isStale` (`cre/lib/isStale.ts` — keep in sync with `src/isStale.ts`) and `quoteFromFeed` (`cre/lib/quote.ts` — keep in sync with `src/quote.ts`).
- Fail closed on RPC/read/zero/negative/stale/future. No EVM write, no wallet, no `--broadcast`, no DON deploy.
- Output JSON same shape as CLI `--json` plus `allowExecute` + `execute` dry-run note.

Source feed: https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet
Decimals fetched via `decimals()` — not hardcoded.

## Prereqs

- CRE CLI `cre` v1.29+ (`cre version`)
- Config in `cre/workflows/stale/config.staging.json` (staging) and `config.production.json`:

```json
{
  "schedule": "0 */5 * * * *",
  "chainName": "ethereum-mainnet",
  "feed": "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419",
  "maxAgeSeconds": 60,
  "amountEth": 0.5
}
```

- RPC for `ethereum-mainnet` in `cre/project.yaml` (`rpcs:`) — uses `https://1rpc.io/eth` (tried `https://ethereum-rpc.publicnode.com` first). Required, no default.

## Simulate (local, no DON deploy)

From repo root:

```bash
# help (verifies cre is installed)
cre workflow simulate --help

# simulate the stale workflow (cron trigger, as template uses)
cre workflow simulate ./cre/workflows/stale --config ./cre/workflows/stale/config.staging.json --allow-insecure-rpc
```

Simulate on public RPCs (publicnode, 1rpc.io) compiled then failed or hung (context canceled / 120s timeout). No DON deploy. No broadcast.

- Trigger is `cron` (`schedule` in config) — one trigger only, as `read-data-feeds-ts` template. No extra HTTP trigger.
- `--allow-insecure-rpc` needed for publicnode HTTP RPC (non-localhost).
- Do NOT use `--broadcast` — simulation only, no transaction.
- For HTTP entrypoint workflows, use `--http-payload '{"maxAgeSeconds":60}'` — but stale uses cron, so no payload.

Expected output is JSON stringified result:

```json
{"decision":"ALLOW","reason":"fresh: age 10s <= maxAge 60s — ALLOW","feed":"0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419","answer":"300000000000","priceUsd":3000,"amountEth":0.5,"quoteUsd":1500,"updatedAt":"...","ageSeconds":10,"maxAgeSeconds":60,"now":...,"allowExecute":true,"execute":{"action":"none","note":"v3 dry-run. no tx. no Agents call."}}
```

If `allowExecute` is `false`, `execute` is `{"action":"none","note":"skipped — BLOCK"}` and workflow still returns `BLOCK` (do not act).

## Layout

```
cre/
  project.yaml                 # rpcs for ethereum-mainnet (copied keys from read-data-feeds-ts template)
  contracts/evm/ts/generated/PriceFeedAggregator.ts
  lib/isStale.ts               # keep in sync with src/isStale.ts
  lib/quote.ts                 # keep in sync with src/quote.ts
  workflows/stale/
    main.ts                    # Runner entry
    workflow.ts                # onCron + initWorkflow (chain-read only)
    workflow.yaml              # staging/production workflow-name + workflow-path/config-path (copied keys)
    config.staging.json
    config.production.json
    package.json               # @chainlink/cre-sdk, viem, zod
    tsconfig.json
```

## Verify

```bash
npm test                       # 19 tests in src/*.test.ts must still pass
cre workflow simulate --help   # must exist, do not fake success
cre workflow simulate ./cre/workflows/stale --config ./cre/workflows/stale/config.staging.json --allow-insecure-rpc
```

No Aave, CCIP, CRE write, Agents, x402, keys, or live DON calls.

## Related

Root `README.md` → `CRE simulation` section links here.
