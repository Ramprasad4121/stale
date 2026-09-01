# Contributing to stale

Guardrail for onchain agents — PRs only.
Repo docs: `README`/`README.md`.
Package: `npm install @ramprasad4121/stale` — `import { isStale } from "@ramprasad4121/stale"`, `npx stale`, `npx stale-mcp`.

- **PRs only, not direct to `main`** — `opencode-review` runs on PRs (`opened`, `synchronize`, `reopened`, `ready_for_review`). Push to a branch, open a PR against `main`.
- **Tests must stay green** — `npm test` (33: 10 isStale + 7 quote + 11 checkPrice + 5 MCP in `src/*.test.ts` + `src/mcp/*.test.ts`, run `npm test` for current count) must pass. No live RPC in tests (mock `viem` `readContract`/`getChainId` and MCP `stdio`).
- **No secrets** — no private keys, wallet, `.env`, or `AGENTS.md`/`LINUS.md` in git (both are `.gitignore` local-only).
- **Official Chainlink fields only, fail closed** — `latestRoundData` + `decimals()` (don’t hardcode decimals), `updatedAt == 0` / `answer <= 0` / missing / stale / future → `BLOCK`. See `isStale`, `quoteFromFeed`, `checkPrice`.
- **Feed allowlist** — only ETH/USD and BTC/USD mainnet are listed (`src/feeds.ts`). Unknown or unsupported feed addresses → `BLOCK`.
- **CRE simulation** — from `cre/`:
  ```bash
  cre workflow simulate workflows/stale --target staging-settings --non-interactive --trigger-index 0 --allow-insecure-rpc
  ```
  Single `cron` trigger (as `read-data-feeds-ts` template). No `--broadcast`, no `cre workflow deploy` unless a human says so.
- **Do not deploy** — no `cre workflow deploy`, no DON, no tx, no wallet.

Staging `maxAgeSeconds` 3600 for simulate `ALLOW`; production stays 60.
