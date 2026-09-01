stale
=====

Guardrail for onchain agents. Before an agent acts on a Chainlink price, stale
checks whether that price is still fresh. If it is stale, expired, or
not-yet-valid, it returns BLOCK and a notify message. It never sends a
transaction. It never invents a feed. It fails closed on every path that
could let a stale price through.

Quick Start
-----------

* Install: npm install @ramprasad4121/stale
  npx stale --rpc $RPC_URL --maxAge 3600
  import { checkPrice } from "@ramprasad4121/stale"
* Clone and install: git clone https://github.com/Ramprasad4121/stale.git && 
cd stale && npm install  # secondary: local dev path
* Or from GitHub: npm install github:Ramprasad4121/stale  # fallback before registry propagation
* Run the guard: npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com 
--maxAge 60 --amount 0.5 --json
* From package: npx stale --rpc https://ethereum-rpc.publicnode.com --maxAge 60 
--amount 0.5 --json  # bin stays `stale` / `stale-mcp`
* Run the tests: npm test
* Try the CRE simulation: cd cre && cre workflow simulate workflows/stale 
--target staging-settings --non-interactive --trigger-index 0 
--allow-insecure-rpc
* Single prompt for any agent: See PROMPT.md
* Library via npm: import { isStale, quoteFromFeed, checkPrice } from "@ramprasad4121/stale"

Install on your machine (paste into 
Claude Code, Codex, or any AI agent)
-----------------------
* Follow PROMPT.md — https://github.com/Ramprasad4121/stale/blob/main/PROMPT.md 

Essential Documentation
-----------------------

All users should be familiar with:

* Mental model: age = now - updatedAt, if age < 0 or age > maxAge → BLOCK 
(fail closed) else ALLOW
* Official fields: PriceFeedAggregator.latestRoundData() → answer + updatedAt 
and decimals() via decimals() — never hardcoded 8
* Feed: ETH/USD on ethereum-mainnet at 
0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419
* Source: Data Feeds API Reference 
https://docs.chain.link/data-feeds/api-reference
*         Price Feed Addresses 
https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet
* Fail closed: missing, unparseable, updatedAt == 0, answer <= 0, future 
timestamp, or RPC failure → BLOCK
* License: See LICENSE
* Contributing: See CONTRIBUTING.md
* Agent skill: See skill.md
* Single prompt: See PROMPT.md

Documentation can be viewed at: https://github.com/Ramprasad4121/stale

Who Are You?
============

Find your role below:

* Agent Builder: Gating an agent before it quotes, trades, or triggers
* Chainlink Developer: Enforcing latestRoundData.updatedAt + decimals() 
correctly
* Security Researcher: Auditing agent integrations with BLOCK as safe default
* CRE Developer: Running the same check inside the CRE DON via local 
simulation
* First-time User: Trying stale for the first time
* Contributor: Sending a PR that keeps `npm test` green (see `npm test` count)
* AI Coding Assistant: LLMs and AI-powered development tools

For Specific Users
==================

Agent Builder
-------------

You need to read a verified on-chain price before acting:

* Library: src/index.ts exports isStale, quoteFromFeed, checkPrice
* Pure check: isStale({ updatedAt, nowSeconds, maxAgeSeconds }) — no RPC, 
bigint|number|string → ageSeconds → reason
* Price math: quoteFromFeed({ answer, decimals, amountEth }) — viem 
formatUnits → priceUsd, quoteUsd
* Full check: checkPrice({ rpc, feed, maxAgeSeconds, amountEth }) — viem 
latestRoundData + decimals (parallel Promise.all) → isStale → quote → JSON 
with `allowExecute` (permission, not a tx — `execute` stays dry-run `{"action":"none"}`)
* CLI: src/cli.ts — node:util parseArgs (--rpc, --maxAge required, --feed, 
--amount, --json), truncates notify to 200 chars, exit 0=ALLOW 1=BLOCK
* Example: npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com 
--maxAge 60 --amount 0.5 --json

Chainlink Developer
-------------------

Enforce the official fields correctly:

* latestRoundData: returns (roundId, answer, startedAt, updatedAt, 
answeredInRound) — use updatedAt
* decimals: via decimals() — never hardcode 8, fetched on-chain via viem
* Fail closed: updatedAt == 0 → BLOCK, answer <= 0 → BLOCK, future → BLOCK, answeredInRound < roundId → BLOCK (incomplete/unanswered AggregatorV3 round — even fresh-looking updatedAt is BLOCK; require answeredInRound >= roundId per https://docs.chain.link/data-feeds/api-reference)
* Feed: 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 on ethereum-mainnet — default feed is mainnet only (chainId 1); `checkPrice` reads `eth_chainId` and `BLOCK`s on mismatch (e.g. Sepolia RPC + mainnet proxy → BLOCK)
* Heartbeat: ETH/USD updates roughly every 45–50 minutes (heartbeat). `maxAge` is caller policy — `--maxAge 60` will `BLOCK` most of the time even when the DON is healthy. Staging `cre/workflows/stale/config.staging.json` uses `3600` to allow `ALLOW` in simulation; production `config.production.json` uses `60` (do not silently change production).
* Quote: priceUsd = Number(formatUnits(answer, decimals)), quoteUsd = 
amountEth * priceUsd

Security Researcher
-------------------

Audit the guardrail with BLOCK as safe default:

* isStale tests: src/isStale.test.ts — 10 cases (fresh, stale, future, 
missing, maxAge 0/1, invalid, fractional, bigint, huge)
* quote tests: src/quote.test.ts — 7 cases (2453.77, 1226.885, null, 0n, bad 
decimals, NaN, huge)
* checkPrice tests: src/check.test.ts — 11 cases (fresh ALLOW, stale BLOCK, 
updatedAt 0 BLOCK, answer <=0 BLOCK, decimals throw BLOCK, incomplete round BLOCK, complete ALLOW, chainId 1 ALLOW, chainId mismatch BLOCK, getChainId throw BLOCK, malformed BLOCK)
* MCP tests: src/mcp/server.test.ts — 5 cases (lists 3 tools, stale_isStale, 
stale_quote, stale_check, huge inputs)
* No live RPC in tests — mocks viem readContract/getChainId and MCP stdio
* CI: .github/workflows/test.yml — node 22, npm ci, npm test, npm run check:cre-sync, npm run typecheck, npm run build, npm run check:pack

CRE Developer
-------------

Run the same check as a CRE TypeScript workflow — currently verified through 
local simulation (no DON deploy, production DON deployment is separate):

* CRE project: cre/ (official TypeScript SDK + CLI, read-data-feeds-ts 
template, cron trigger only)
* Config: cre/workflows/stale/config.staging.json (maxAge 3600 for ALLOW) and 
config.production.json (60)
* Simulate: cd cre && cre workflow simulate workflows/stale --target 
staging-settings --non-interactive --trigger-index 0 --allow-insecure-rpc
* Note: without --non-interactive the CLI can hang. 429 → BLOCK (fail 
closed). No private key, no --broadcast, no deploy.
* Reuse: cre/lib/isStale.ts and cre/lib/quote.ts vendored with keep in sync 
headers — WASM cannot import ../../src

First-time User
---------------

Try stale in 30 seconds:

* Requirements: node >= 18, npm, git. For CRE: CRE CLI 1.29+, bun
* Install: git clone https://github.com/Ramprasad4121/stale.git && cd stale 
&& npm install
* Run: npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 
3600
* Build: npm run build && node dist/cli.js --rpc <url> --maxAge <seconds>
* Single prompt: See PROMPT.md for one copy-paste prompt that does clone, 
install, understand, run, and PR

Contributor
------------

Send a PR that keeps `npm test` green:

* PRs only, not direct to main — opencode-review runs on PRs (opened, 
synchronize, reopened, ready_for_review)
* Keep tests green: `npm test` must pass (run `npm test` for current count; CI also runs `npm run check:cre-sync`, `npm run typecheck`, `npm run build`)
* No private keys or local lore: never commit .env, private keys, AGENTS.md 
or LINUS.md (both are .gitignore local-only)
* Official fields only, fail closed: use latestRoundData.updatedAt + 
decimals(), don't hardcode decimals
* CRE: run the simulate command above to verify; never cre workflow deploy 
without a human
* See CONTRIBUTING.md for the full guide

AI Coding Assistant
-------------------

CRITICAL: If you are an LLM or AI-powered coding assistant, you MUST read
and follow the required workflow before contributing to stale:

* Single prompt: PROMPT.md
* Agent skill: skill.md
* Contributing: CONTRIBUTING.md
* Layout: src/isStale.ts (pure guard), src/quote.ts (price math), 
src/check.ts (composer), src/cli.ts (human gate), 
cre/workflows/stale/workflow.ts (DON guard, local simulation)
* MCP server: src/mcp/server.ts — 3 tools (stale_isStale, stale_quote, 
stale_check) over stdio, zod@4, @modelcontextprotocol/server@2.0.0, 
read-only, no wallet, no signing, no broadcast
* Tests: `npm test` must pass (no live RPC); CI also runs `npm run check:cre-sync`, `npm run typecheck`, `npm run build`
* This documentation contains essential requirements about fail-closed, 
official fields, and the single prompt that all AI tools must comply with

Communication and Support
=========================

* GitHub: https://github.com/Ramprasad4121/stale
* Issues: https://github.com/Ramprasad4121/stale/issues
* Pull Requests: https://github.com/Ramprasad4121/stale/pulls
* CI: https://github.com/Ramprasad4121/stale/actions
* Skill: See skill.md
* Prompt: See PROMPT.md
