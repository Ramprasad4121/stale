# Contributing to stale

Guardrail for onchain agents — PRs only.

- **PRs only, not direct to `main`** — `opencode-review` runs on PRs (`opened`, `synchronize`, `reopened`, `ready_for_review`). Push to a branch, open a PR against `main`.
- **Tests must stay green** — `npm test` (19 in `src/*.test.ts`) must pass. No live RPC in tests (mock `viem` `readContract`).
- **No secrets** — no private keys, wallet, `.env`, or `AGENTS.md`/`LINUS.md` in git (both are `.gitignore` local-only).
- **Official Chainlink fields only, fail closed** — `latestRoundData` + `decimals()` (don’t hardcode decimals), `updatedAt == 0` / `answer <= 0` / missing / stale / future → `BLOCK`. See `isStale`, `quoteFromFeed`, `checkPrice`.
- **CRE simulation** — from `cre/`:
  ```bash
  cre workflow simulate workflows/stale --target staging-settings --non-interactive --trigger-index 0 --allow-insecure-rpc
  ```
  Single `cron` trigger (as `read-data-feeds-ts` template). No `--broadcast`, no `cre workflow deploy` unless a human says so.
- **Do not deploy** — no `cre workflow deploy`, no DON, no tx, no wallet.

Staging `maxAgeSeconds` 3600 for simulate `ALLOW`; production stays 60.
