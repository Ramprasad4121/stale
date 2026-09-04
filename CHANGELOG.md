# stale Changelog

## 2.0.0 — 2026-09-04

Major release: the post-audit, certification-hardened guardrail suite. Every section below shipped behind green CI (tests, clippy `-D warnings`, fmt, docs, CodeQL, review bot) plus red/blue/docs-team adversarial review and live-mainnet validation.

### Migrating from 1.x

Signature changes (compiler guides you — fix every call site):

- `encode_address_param(addr) -> String` is now `-> Result<String, String>`. Map `Err` to `BLOCK`; never `unwrap`.
- `check_sanctioned(client, address)` is now `check_sanctioned(client, chain_id: u64, address)`. Pass `1` for mainnet; any other chain `BLOCK`s.
- `check_balance(client, agent, token, required)` is now `check_balance(client, agent, token, required, gas_reserve_wei: u128)`. Pass `0` for legacy exact-solvency semantics.
- `SimulateTxInput { account, to, data }` gains `value: Option<u128>`. Pass `None` for legacy 0-value simulation; set `Some(wei)` for payable flows.
- `PipelineResult` gains `guards_skipped: Vec<String>` (omitted from JSON when empty). Additive — no call-site change needed.
- `EvmRpcClient` gains `get_base_fee`, `get_priority_fee`, `call_from_with_value` with fail-closed `Err` defaults. External transports compile unchanged; implement the methods to opt into 1559/value simulation.

Behavioral tightenings (no signature change — audit your policies):

- `check_paused`: ANY revert now `BLOCK`s (was: revert → `ALLOW`).
- `decode_bool` accepts only `0`/`1`; any other word is `Err` → `BLOCK`.
- `slippage_bps == 10000`, `check_approval(0)`, zero pool minimums, and required-0-plus-reserve-0 solvency checks now `BLOCK` as vacuous.
- `RateLimiter::new` rejects `max_tx > 100_000`; flood-evicted histories `BLOCK` until the window drains.
- Deviation threshold must satisfy `0 < max ≤ 100`; the verdict is exact integer math (`f64` display-only).
- `stale is-stale` exits `1` on `BLOCK`; negative `--max-age` exits `2`.
- `stale-mcp` requires explicit `decimals`/`nowSeconds`/`maxAgeSeconds`; every rejection is JSON with `isError: true`.

## Unreleased

(nothing — cut 2.0.0 above; add new entries here.)

### Breaking Changes

- `encode_address_param` now returns `Result` and rejects non-40-hex-char input instead of silently truncating (PR #41).
- `slippage_bps == 10000` (100%) is rejected; it would allow total loss (PR #41).
- `check_sanctioned` takes a `chain_id` argument and BLOCKs on any chain other than mainnet, where the oracle lives (PR #44).
- `stale is-stale` exits 1 on BLOCK, matching `stale check` (PR #41).
- `stale-mcp stale_quote` requires explicit `decimals` (no silent default) and caps at the on-chain maximum (PR #41).
- `stale-mcp stale_isStale` errors on missing `nowSeconds`/`maxAgeSeconds` instead of defaulting to 0 (PR #41).
- `AuditLogger::record` secret-scrubs reasons and metadata (embedded credentials become `<redacted>`) (PR #44).

### Breaking Changes (round 3)

- `check_paused`: ANY revert now BLOCKs, including "contract does not implement `paused()`". The previous revert→ALLOW carve-out was fail-open in isolation (a malicious contract can `revert` to force ALLOW). Compose with `AddressBook` + `check_is_contract` so only known pausable contracts reach this guard.
- `PipelineResult` gains `guards_skipped: Vec<String>` (omitted from JSON when empty): `FailFast` names every guard it did not execute, so audit trails never silently omit unevaluated guards.

### Security Fixes

- Examples + integration test taught the check-snapshot anti-pattern (#52): `examples/guard_pipeline.rs` and `tests/integration_pipeline.rs` snapshotted `RateLimiter::check()` / `SpendingCap::check()` once and replayed the verdict, so the governors could never trip. Guards now call `try_acquire` / `try_spend` live inside `Arc<Mutex<…>>`; the integration test proves it by tripping the 5 ETH cap on the 6th preflight.
- `check_prices` batch truncation (#54): inputs past the cap were silently dropped; every input now yields exactly one row, excess entries each getting an explicit BLOCK row.
- Sanctions chain binding (#56): `check_sanctioned` verifies the `chain_id` argument against the transport's `eth_chainId` — argument/RPC mismatch or unverifiable chain → BLOCK instead of querying a non-oracle address.
- Small correctness bundle (#59): `check_approval` boundary is now inclusive (`>= 2^126` BLOCKs, matching docs); `AddressBook` trims on insert/lookup/`has`/`label_of`; ABI word-offset math is checked (huge offsets → Err, never panic); mock documents address blindness; `check`/`remaining` docs no longer claim non-mutating; slippage `MAX_EXP_DIFF` comment corrected.
- MCP argument coercion (#58): `stale_quote`/`stale_check`/`stale_isStale` accept JSON numbers and numeric strings uniformly (`answer`, `decimals`, `amountEth`, `nowSeconds`, `maxAgeSeconds`); present-but-wrong-typed args report `invalid` instead of `missing`, present-but-invalid `amountEth` BLOCKs instead of silently dropping to `quoteUsd: null`, and `stale_quote` success carries explicit `isError: false`.
- SSRF redirect bypass (#51): `HttpRpcClient` never follows redirects (`Policy::none`); any `3xx` → BLOCK, so a 307 cannot smuggle egress past the RPC allowlist. Covered by a zero-egress regression test.
- Audit scrub gaps (#57): `scrub_secrets` is now ASCII case-insensitive with an extended key list (`password`, `private_key`, `mnemonic`, `bearer`, …) plus `Bearer`/`Basic` token masking; `GuardPipeline` reports scrub reasons identically so serialized reports cannot re-leak.
- Pipeline timeout task leak: expired guards are now `abort()`ed instead of running detached past the verdict.
- Sequencer coverage gaps (#55): Blast / Linea / Arbitrum Nova have centralized sequencers but no configured uptime feed — `check_sequencer` BLOCKed loudly there instead of silently passing. No feed addresses invented; adding them is a registry change.

- PR #41: `is_stale`/`deadline` overflow → BLOCK; `updatedAt > i64::MAX` → BLOCK; negative `now` → BLOCK; decimals validity cap; sequencer unexpected-answer/incomplete-round → BLOCK; `pausable` malformed-response and RPC-error misclassification → BLOCK; slippage checked math; deviation self-comparison/stale-round → BLOCK; RPC 10s timeout, HTTPS-only (except localhost), URL redaction; `decode_round_data` exact-length check.
- PR #43: RPC parsed-host validation, 1 MiB response cap; pipeline per-guard timeout + panic isolation + `MAX_GUARDS`; atomic rate-limiter acquisition + history cap; audit FIFO; nonce exact-equality; `check_prices` bounded at 64; release/dev `overflow-checks=true`.
- PR #44: honeypot `false`-return → BLOCK (fee-on-transfer); sanctions chain gate; audit-log secret scrub; zero `unwrap()` in binaries.

### New Guards

- `check_gas_price_1559(client, max_base_fee_gwei, max_priority_fee_gwei)`: EIP-1559 circuit breaker enforcing base fee and tip independently. New `EvmRpcClient::{get_base_fee, get_priority_fee}` have fail-closed default bodies (`Err`), so external transports are never silently treated as 1559-capable.

### Docs Drift Fixes

- README dependency pin `1.0.2` → `2.0.0`; registry scope corrected (15 feeds / 9 chains); `check_price_deviation` library-only gap documented; `check_is_contract` proxy caveat; CLI exit-code contract (`0` ALLOW / `1` BLOCK / `2` usage) documented; CRE pointer added. CLI `--help` descriptions filled; legacy top-level path plumbs `--allowed-rpc-hosts`; usage errors exit `2`. `FeedEntry` serializes camelCase (`chainId`). Live example uses 3600s max-age; pool/solvency/gas checks warn (not asserts) on flaky live state, while identity/compliance checks (feed, sanctions, EOA, contract, pause, sequencer) keep hard asserts — the canary depends on them to alert. `cre/README` paths point at the Rust sources; `cre` package versions aligned to `2.0.0`.

## 1.0.2

- Real-world mainnet audit fixes over 1.0.1.

## 1.0.1

- Resolved all audit findings and hardened fail-closed invariants over 1.0.0.

## 1.0.0

### Major Changes

- 181ff1b: Version 1.0.0 is here! Added a massive README overhaul detailing the 10-Point Defense System. `stale` is officially the ultimate Linux-level security standard for AI agents.
- 🚀 Version 1.0.0 Release

  We are proud to present `stale` v1.0.0 — the complete, production-ready, fail-closed guardrail suite for autonomous onchain AI agents.

  This major release solidifies the **10-Point Defense System**, including:

  1. **Chainlink Oracle Guardrails** (stale data, multi-oracle deviation, sequencer liveness)
  2. **Liquidity & Slippage Guardrails** (DEX pool depth validation, exact MEV protection bounds)
  3. **Approval & Access Guards** (strict spending allowances, protocol pausable detection)
  4. **Network & State Guards** (gas spikes, stale intent deadlines, nonce desync, chain ID mismatch)
  5. **Entity & Phishing Guards** (EOA bytecode checks, Token Tax / Honeypot simulation)
  6. **Compliance Guards** (Chainlink OFAC Sanctions Oracle)
  7. **Infrastructure Guards** (MEV-protected private RPC enforcement)
  8. **Resource Guards** (In-memory Rate Limiting and Spending Caps)
  9. **Access Control** (AddressBook strict allowlists)
  10. **Agent Solvency** (Balance sufficiency pre-checks)

  All backed by a robust `GuardPipeline` with fluent composability and `AuditLogger` for full compliance trails.

  Ready for production. Make no mistakes.

### Minor Changes

- 678ec32: Introduced `checkApproval`, a strict allowance guardrail. Prevents AI agents from executing `approve` transactions with `MaxUint256` or dangerously large bounds, enforcing "Exact Amount Approvals" to prevent complete treasury drains on compromised routers.
- 77623a1: Introduced three powerful new guardrails:

  - `checkPriceDeviation`: Multi-oracle price comparison. Queries two independent Chainlink feeds for the same asset and blocks if they deviate beyond a configurable threshold — the gold-standard defense against flash-loan oracle manipulation.
  - `checkDeadline`: Ensures swap deadlines are reasonable. Blocks if the deadline is expired (stale intent replay), too tight (will expire before confirmation), or too far in the future (long-lived MEV risk).
  - `AddressBook`: Configurable contract allowlist. If an address is not explicitly approved, the agent cannot interact with it. The simplest and most powerful access control.

- 3a4df00: Added L2 Sequencer Uptime feeds and ETH/USD price feeds for Scroll, zkSync Era, Mantle, and Metis Andromeda.
- b7102a1: Introduced `checkGasPrice`, a network gas spike circuit breaker. It actively polls the base fee over RPC and fails closed (blocks execution) if network congestion causes gas prices to spike beyond the agent's safe threshold. This prevents AI agents from unknowingly burning disproportionate amounts of their treasury on transaction fees during gas wars.
- c67b724: Introduced `checkTokenTax` — a honeypot token detection guardrail. Uses `eth_call` to simulate a token transfer from a known holder. If the transfer reverts (meaning the token cannot be sold/transferred), the token is flagged as a honeypot and execution is blocked. This is the #1 defense against the most common DeFi scam vector.
- 57ddfad: Introduced native on-chain DEX liquidity guardrails `checkPoolV2` and `checkPoolV3`. These strictly protect AI agents from executing trades in low-liquidity honeypot pools by evaluating active depth natively over RPC before a transaction is signed.
- 6452059: Introduced `checkMevRpc` and `checkIsContract` guardrails.
  `checkMevRpc` ensures an agent is strictly using a known private, MEV-protected RPC endpoint before trading, preventing front-running and sandwich attacks in the public mempool.
  `checkIsContract` ensures an agent is strictly interacting with a deployed smart contract, preventing EOA phishing attacks where agents are tricked into approving or sending funds to scammer wallets.
- b3b107c: Introduced `checkPrices` for highly optimized multicall batching, drastically reducing RPC calls when checking multiple feeds. `checkPrice` now uses `checkPrices` internally.
- 028c8b0: Introduced a robust Network State Guardrail suite.

  - `checkRpcSync`: Fails closed if the agent's RPC node falls out of sync, preventing the agent from acting on deeply stale or fake blockchain states.
  - `checkChainId`: Strictly enforces that the RPC matches the agent's expected execution environment, preventing catastrophic cross-chain replay attacks.
  - `checkNonce`: Actively queries the network for the agent's nonce, blocking execution if the agent is desynced and trying to double-spend or re-use a stale nonce.

- 197b38d: Introduced `checkSanctioned` OFAC Compliance Guardrail. `stale` now utilizes the official Chainlink Sanctions Oracle to ensure that autonomous AI agents never interact with sanctioned entities (e.g. Lazarus Group, Tornado Cash routers), protecting node operators and developers from severe compliance violations.
- 76acd11: Introduced two game-changing integration features:

  - `createGuardPipeline()`: A composable pre-flight pipeline that chains multiple guardrails into a single `run()` call. Supports fail-fast and run-all modes, async guards, per-guard timing, and fluent chaining. This is the recommended way to integrate stale into production agents.
  - `AuditLogger`: A structured compliance-grade audit logger that records every ALLOW/BLOCK decision with timestamps, reasons, and metadata. Supports FIFO eviction, filtering, JSON export, and external callbacks (e.g. ship to Datadog/Splunk).

- 71ec820: Refactored sequencer logic into standalone module and integrated automated release management via Changesets.
- e8067fd: Introduced `RateLimiter` and `SpendingCap` guardrails. These are pure in-memory, zero-dependency classes that enforce transaction frequency limits and cumulative spending caps over rolling time windows, preventing runaway AI agents from draining wallets or DOSing protocols.
- 981c817: Introduced `simulateTx` and `checkAllowance` guardrails.
  `simulateTx` natively executes `eth_call` to trace an agent's intended transaction against the current blockchain state, failing closed if it reverts (preventing honeypots and wasted gas).
  `checkAllowance` ensures an agent actually has the required ERC20 allowance granted to a spender before attempting a trade, preventing reverts from insufficient approvals.
- 534075c: Introduced `calculateMinAmountOut`, a native bigint slippage engine. It utilizes Chainlink Data Feeds to dynamically compute safe, mathematically guaranteed exact `minAmountOut` boundaries for DEX swaps, completely mitigating MEV sandwich attacks on autonomous agents.
- 5c673cc: Introduced `checkBalance` (Solvency Guardrail) and `checkPaused` (Protocol State Guardrail). Agents can now natively verify they hold sufficient funds before attempting trades, preventing wasted gas and network spam from failing transactions. Additionally, agents can natively verify that target protocols (e.g., USDC, Aave) are not paused by their multi-sigs before attempting interaction.
