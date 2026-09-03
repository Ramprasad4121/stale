# Contributing to stale

Fail-closed Rust guardrail for onchain agents — PRs only.

- **PRs only, not direct to `main`** — `opencode-review` runs on PRs (`opened`, `synchronize`, `reopened`, `ready_for_review`). Push to a branch, open a PR against `main`.
- **Tests must stay green** — `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`, and `cargo doc --no-deps` (with `RUSTDOCFLAGS: -D warnings`) must pass. No live RPC in tests (use `MockRpcClient` in `src/mock.rs`); no tx paths, no wallets.
- **No secrets** — no private keys, wallets, `.env`, or `AGENTS.md`/`LINUS.md` in git.
- **Official Chainlink fields only, fail closed** — `latestRoundData` + `decimals()` (never hardcode decimals), `updatedAt == 0` / `answer <= 0` / missing / stale / future / incomplete round → `BLOCK`. See `src/is_stale.rs`, `src/quote.rs`, `src/check.rs`.
- **Feed allowlist** — `check_price` only queries feeds in `src/feeds.rs` (`REGISTRY`). Unknown or unsupported feed addresses → `BLOCK`.
- **Fail-closed deltas only** — new `EvmRpcClient` methods need fail-closed default bodies (`Err` → caller emits `BLOCK`); new guards need reason strings, unit tests (allow + breach + missing-feed + zero-policy), `lib`/`prelude` re-exports, README + CHANGELOG + SECURITY.md updates.
- **Do not deploy or broadcast** — no `eth_sendTransaction`, no signing, no wallet. `simulate_tx` is `eth_call` dry-run only.
