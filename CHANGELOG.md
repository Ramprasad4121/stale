# @ramprasad4121/stale

## Unreleased

### Breaking Changes

- `encode_address_param` now returns `Result` and rejects non-40-hex-char input instead of silently truncating (PR #41).
- `slippage_bps == 10000` (100%) is rejected; it would allow total loss (PR #41).
- `check_sanctioned` takes a `chain_id` argument and BLOCKs on any chain other than mainnet, where the oracle lives (this PR).
- `stale is-stale` exits 1 on BLOCK, matching `stale check` (PR #41).
- `stale-mcp stale_quote` requires explicit `decimals` (no silent default) and caps at the on-chain maximum (PR #41).
- `stale-mcp stale_isStale` errors on missing `nowSeconds`/`maxAgeSeconds` instead of defaulting to 0 (PR #41).
- `AuditLogger::record` secret-scrubs reasons and metadata (embedded credentials become `<redacted>`) (this PR).

### Security Fixes

- PR #41: `is_stale`/`deadline` overflow → BLOCK; `updatedAt > i64::MAX` → BLOCK; negative `now` → BLOCK; decimals validity cap; sequencer unexpected-answer/incomplete-round → BLOCK; `pausable` malformed-response and RPC-error misclassification → BLOCK; slippage checked math; deviation self-comparison/stale-round → BLOCK; RPC 10s timeout, HTTPS-only (except localhost), URL redaction; `decode_round_data` exact-length check.
- PR #43: RPC parsed-host validation, 1 MiB response cap; pipeline per-guard timeout + panic isolation + `MAX_GUARDS`; atomic rate-limiter acquisition + history cap; audit FIFO; nonce exact-equality; `check_prices` bounded at 64; release/dev `overflow-checks=true`.
- This PR: honeypot `false`-return → BLOCK (fee-on-transfer); sanctions chain gate; audit-log secret scrub; zero `unwrap()` in binaries.

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
