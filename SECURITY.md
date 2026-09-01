# Security Policy for stale

`stale` is a fail-closed guardrail. It does not make economic decisions and it
does not execute transactions. This document describes what it trusts, what it
does not trust, and how it fails.

## Summary

`stale` answers one question: is the Chainlink price this agent is about to
act on fresh enough under the caller's `maxAgeSeconds` policy?

- `ALLOW` means the price passed every freshness and validity check.
- `BLOCK` means do not proceed. Notify the human and stop.

`allowExecute` is permission state only. It is not an execution mechanism.

## Trust boundaries

- **Trusted:** Chainlink Data Feed proxies on the allowlist in `src/feeds.ts`
  (ETH/USD and BTC/USD, Ethereum mainnet only), their
  `PriceFeedAggregator` `latestRoundData()` and `decimals()`, the `viem`
  `PublicClient` `eth_call` path, and the local clock (`nowSeconds`).
- **Untrusted:** RPC responses, `updatedAt`, `answer`, `decimals`, `feed`
  address strings, `maxAgeSeconds`, `amountEth`, and any human or agent that
  calls `checkPrice` with a permissive policy. All are validated and fail
  closed.
- **Feed allowlist:** only ETH/USD `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419`
  and BTC/USD `0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c` on Ethereum
  mainnet are supported (source:
  https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet).
  `checkPrice` fails closed on any valid-format address not in the registry
  (`unknown/unsupported feed`). Adding a feed is a registry change with
  tests and this doc updated.

## Chainlink assumptions

- Data Feeds: `latestRoundData()` returns
  `(uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt,
  uint80 answeredInRound)` and `decimals()` returns `uint8`.
  `updatedAt` is seconds since epoch (uint80, but fits in 53 bits for the
  `age = now - updatedAt` math). `answer` is `int256` scaled by `10**decimals`.
- Never hardcode `decimals`. `stale` fetches it on-chain via `decimals()`
  and uses `viem` `formatUnits(answer, decimals)` for `priceUsd`. Source:
  https://docs.chain.link/data-feeds/api-reference and
  https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet.
- Do not assume `updatedAt` is always increasing, always recent, or always
  present. `updatedAt == 0` means no data yet → `BLOCK`.
- Do not assume `answer` is always positive. `answer <= 0` → `BLOCK`.
- Round completeness (AggregatorV3): `latestRoundData` returns `roundId` and
  `answeredInRound`. If `answeredInRound < roundId` the round is incomplete /
  unanswered → `BLOCK` with `allowExecute:false` even if `updatedAt` looks
  fresh. `stale` (Node `checkPrice` and CRE `onCron`) requires
  `answeredInRound >= roundId` per
  https://docs.chain.link/data-feeds/api-reference; `startedAt` is not used
  for freshness. Remaining policy (heartbeat, deviation) stays in `ROADMAP`.
- Chain binding: every allowlisted feed (ETH/USD `0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419`, BTC/USD `0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c`) is Ethereum mainnet only via chainId 1 in the registry. Node `checkPrice` calls `eth_chainId` (viem `getChainId`) after `createPublicClient({chain: mainnet})` and `BLOCK`s if `chainId !== 1` with `chainId mismatch` reason; mocks without `getChainId` skip the check.
- Heartbeat: ETH/USD on mainnet updates roughly every 45–50 minutes (Chainlink heartbeat). `maxAgeSeconds` is caller policy — `maxAge 60` will `BLOCK` most of the time even when the DON is healthy. Staging `3600` vs production `60` in `cre/workflows/stale/config.*.json` is intentional (simulate `ALLOW` vs strict production); do not silently change production.
- Proxy/aggregator behavior: the proxy forwards to the current aggregator.
  A stale proxy still returns whatever the aggregator last wrote. `stale`
  does not try to detect proxy upgrades — it checks the timestamp it actually
  sees.

## RPC assumptions

- `checkPrice({ rpc })` uses `viem` `createPublicClient({ chain: mainnet,
  transport: http(rpc) })` + `readContract` (`eth_call`). No wallet, no
  signing, no `eth_sendTransaction`.
- Any RPC failure, malformed response, or unexpected structure → `BLOCK`.
  `stale` never replaces a failed read with cached data and never turns an
  operational failure into `ALLOW`.
- Public RPCs (e.g. `https://ethereum-rpc.publicnode.com`, `https://1rpc.io/eth`)
  may `429` or `301`. `429` → `BLOCK` with `reason` containing the RPC error
  (truncated to 200 chars in CLI). Use a private, rate-limited RPC for
  production if you need higher availability.
- `isStale` and `quoteFromFeed` do not touch the network and have no RPC
  assumptions.

## Clock assumptions

- `nowSeconds` defaults to `Math.floor(Date.now() / 1000)`. Clock skew or a
  future `updatedAt` (`age < 0`) → `BLOCK` as `not-yet-valid`. This also
  covers the case where the local clock is behind the chain.
- Staleness is `age = now - updatedAt`. No NTP sync is assumed. If you need
  stronger clock guarantees, run your own time source and pass `nowSeconds`
  explicitly (as `cre` does via `Math.floor(Date.now()/1000)` in the workflow).

## Precision and overflow

- `priceUsd = Number(formatUnits(answer, decimals))`. `answer` is `int256`,
  `decimals` is `0-36`. `formatUnits` returns a decimal string, then `Number`
  may lose precision for very large `answer` (beyond 53 bits). For `ETH/USD`
  `answer ≈ 2.5e11` (8 decimals, ~$2500) this is safe. For feeds with larger
  `answer` or `decimals`, test the conversion and consider keeping `answer`
  as `bigint` until you need `priceUsd`.
- `quoteUsd = amountEth * priceUsd` where `amountEth` is a JS `number`.
  `amountEth` must be finite and `>= 0`; else `quoteFromFeed` throws and the
  caller gets `BLOCK`. `NaN`, `Infinity`, negative, or huge values are
  rejected. No overflow check beyond `Number.isFinite`.
- `priceUsd` and `quoteUsd` are convenience display values in v1, not
  settlement-grade amounts. They use `formatUnits` followed by JavaScript
  `Number` arithmetic; exact decimal-string math is deferred to a later PR.

## Configuration risks

- `maxAgeSeconds` is required, no default. The caller must choose. A permissive
  `maxAgeSeconds` (e.g. `86400`) will `ALLOW` a price that is a day old.
  Staging is `3600` so a local `cre workflow simulate` can `ALLOW` (on-chain
  age is ~2800s); production stays `60`. Document your policy and test it.
- `feed` must be `0x` + 40 hex and must be on the allowlist
  (ETH/USD or BTC/USD mainnet from `src/feeds.ts`). An attacker-controlled
  `feed` can return any `answer`/`updatedAt`; `stale` fails closed on any
  address not in the allowlist, so callers can only pass official proxies.
  Do not accept `feed` from untrusted input.
- `amountEth` is human units. A large `amountEth` with a stale `priceUsd`
  still `BLOCK`s, but a large `quoteUsd` that is then used for economics
  without further checks can cause loss. `stale` only gates freshness.

## Stale-data and malformed-data handling

All of these → `BLOCK` with `ageSeconds: null` or a negative `ageSeconds`:

- missing, `null`, `undefined`, `""`, whitespace-only, or non-numeric `updatedAt`
- `updatedAt` as `bigint`, `number`, or hex/decimal string that fails to parse
- `updatedAt == 0`, `answer <= 0`, `decimals` not integer `0-36`, `amountEth` not finite `>=0`
- `age < 0` (future) → `not-yet-valid`
- `age > maxAgeSeconds` → `stale`
- RPC/read failure, malformed `eth_call` response, unexpected tuple shape

No path silently recovers with a cached price. No path turns an operational
failure into `ALLOW`.

## MCP risks

- MCP server `src/mcp/server.ts` is `stdio` only (`McpServer` +
  `StdioServerTransport` from `@modelcontextprotocol/server@2.0.0`, `zod@4`
  for `inputSchema`). No HTTP, no SSE.
- Tools: `stale_isStale` (pure, no RPC), `stale_quote` (price math, no RPC),
  `stale_check` (full `checkPrice` via `viem`, no wallet). All delegate to
  the same `src/` core (`isStale` → `quoteFromFeed` → `checkPrice`); no
  duplicated freshness logic.
- Read-only: no wallet, no private key, no signing, no `eth_sendTransaction`,
  no `broadcast`, no `Aave`/`CCIP`/`x402`/Agents execution.
- Input validation is `zod` on the MCP boundary; internal `isStale` still
  fail-closed on `bigint|number|string` even if the MCP layer already
  rejected `number`/`bigint` for `updatedAt` (which is `z.string()` only to
  avoid JSON `bigint` loss). This double layer is intentional.

## CLI risks

- `src/cli.ts` uses `node:util` `parseArgs` (`strict:true`,
  `allowPositionals:false`). Unknown flags → error. Missing `--rpc` or
  `--maxAge` → `BLOCK` exit 1. Bad `--amount` → `BLOCK` exit 1.
- Human mode prints `DECISION — reason`, `feed`/`answer`/`updatedAt`/`age`,
  `priceUsd`/`quoteUsd`, then `notify:` and `execute: skipped` or
  `execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}`.
  `--json` prints only the JSON object (same shape as `checkPrice`) and exits
  `0` on `ALLOW`, `1` on `BLOCK`. No JSON pollution.
- Notify errors are truncated to 200 chars to bound `reason` length.

## CRE risks

- `cre/` is a separate CRE project (official TypeScript SDK `1.19.1`, `cron`
  trigger as `read-data-feeds-ts` template, `chain-read` only). It reuses
  `cre/lib/isStale.ts` and `cre/lib/quote.ts` vendored with `keep in sync`
  headers — the CRE WASM build cannot import `../../src`.
- Config: `cre/workflows/stale/config.staging.json` (`maxAgeSeconds: 3600`)
  and `config.production.json` (`60`), `cre/project.yaml` `rpcs:` for
  `ethereum-mainnet`. Staging `3600` allows local `ALLOW`; production `60`
  is the real policy.
- Simulation: `cd cre && cre workflow simulate workflows/stale --target
  staging-settings --non-interactive --trigger-index 0 --allow-insecure-rpc`
  (without `--non-interactive` the CLI can hang; `429` → `BLOCK`; no private
  key, no `--broadcast`, no `cre workflow deploy` without explicit human
  approval). Currently verified through local simulation, not yet running
  inside the DON — production DON deployment is separate.
- No `EVMClient` writes, no wallet, no signing. The workflow returns a JSON
  string via `safeJsonStringify` with `allowExecute` and `execute` dry-run
  note, same shape as CLI `--json`.

## Dependency and supply-chain risks

- Runtime: `viem@2.56.1` (from `^2.21.0`), `zod@4.5.4` (from `^4.0.0`),
  `@modelcontextprotocol/server@2.0.0` + `client@2.0.0` (for MCP stdio tests).
  `cre` pins `@chainlink/cre-sdk@1.19.1`, `viem@2.34.0`, `zod@3.25.76` in
  `cre/workflows/stale/package.json` (separate lockfile).
- Avoid floating `@latest` in security-sensitive paths. `package-lock.json`
  and `cre/bun.lock` / `cre/workflows/stale/bun.lock` pin exact versions.
- Published package (`files: ["dist", "README.md", "LICENSE"]`) contains only
  `dist/` JS/d.ts/maps, `README.md`, `LICENSE`, and `package.json` (verified via
  `npm pack --dry-run`). No `src/*.test.ts`, `AGENTS.md`,
  `LINUS.md`, `.env`, `node_modules`, or `cre` build artifacts.

## Fail-closed semantics

Every public entry — `isStale`, `quoteFromFeed`, `checkPrice`, `stale` CLI,
`stale-mcp` tools, and the CRE `onCron` handler — is fail-closed. The only
way to get `ALLOW` and `allowExecute:true` is to pass *every* check with a
fresh, valid `updatedAt` and `answer`. Any doubt → `BLOCK`.

Never claim stronger guarantees than the implementation. If a new check is
added (e.g. round validation, heartbeat, deviation), it needs a reason, an
implementation, tests, docs, and an update to this file.

## Reporting a vulnerability

Do not open a public issue for a sensitive security report. Instead, open
a draft PR with a failing test that reproduces the `ALLOW` on bad data, or
contact the maintainer via https://ramprasadgoud.dev and describe the
unsafe `ALLOW` path with steps to reproduce.

## License

MIT — see `LICENSE`. This security documentation is part of the project and
is licensed under the same terms.
