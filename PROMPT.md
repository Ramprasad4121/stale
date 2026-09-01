```
# Single Prompt for Any AI Agent — stale


You are an AI agent for github.com/Ramprasad4121/stale — guardrail for onchain agents (ALLOW/BLOCK, quote, dry-run, never tx).

Do this end-to-end, no questions:

1. Clone & install (or install package):
    git clone https://github.com/Ramprasad4121/stale.git && cd stale && npm install
    # or from npm: npm install @ramprasad4121/stale
    # CRE (optional, for simulate): cd cre && bun install && cd cre/workflows/stale && bun install && cd ../../..

2. Understand the project — read in order:
   README (note: now `README` not `README.md`), skill.md, CONTRIBUTING.md, src/isStale.ts, src/quote.ts, src/check.ts, src/cli.ts, src/index.ts, cre/README.md, cre/workflows/stale/workflow.ts, cre/workflows/stale/config.staging.json (maxAge 3600) vs config.production.json (60), .github/workflows/test.yml
   # installed package users: import { isStale } from "@ramprasad4121/stale"; npx stale --help; npx stale-mcp

3. Mental model:
   age = now - updatedAt (from PriceFeedAggregator.latestRoundData + decimals() via viem, never hardcoded 8). If age <0 or age > maxAge → BLOCK (fail closed: missing/0/negative/unparseable/RPC fail → BLOCK). One feed v1: ETH/USD 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 on ethereum-mainnet. Quote: priceUsd=Number(formatUnits(answer,decimals)), quoteUsd=amountEth*priceUsd.

4. Verify:
    npm test  # must pass (33: 10 isStale + 7 quote + 11 checkPrice + 5 MCP, mocked viem/MCP stdio, no live RPC) — run npm test for current count
   npx tsx src/cli.ts --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5 --json
   # or from installed package: npx stale --rpc https://ethereum-rpc.publicnode.com --maxAge 60 --amount 0.5 --json
   # human: --json prints only JSON, default is human lines; exit 0=ALLOW 1=BLOCK; execute: skipped vs v3 dry-run

5. CRE simulate (chain-read only, no wallet, no --broadcast, no deploy):
   cd cre && cre workflow simulate workflows/stale --target staging-settings --non-interactive --trigger-index 0 --allow-insecure-rpc
   # requires cre CLI 1.29+; without --non-interactive it can hang; 429 → BLOCK; staging 3600 → ALLOW (age ~2800s)

6. Ready to PR:
    Create a new branch (not main), make your change in src/ (keep isStale pure, keep cre/lib in sync), keep npm test green (33: 10 isStale + 7 quote + 11 checkPrice + 5 MCP, mocked viem/MCP stdio, no live RPC), never commit AGENTS.md/LINUS.md/.env/private keys, official Chainlink fields only, then open PR against main — opencode-review runs on PR.

You are now ready to run, extend, or review stale. Keep the layout why-not-what: src/isStale (pure), src/quote (math), src/check (composition), src/cli (thin wrapper), cre/ (separate CRE project).
```
