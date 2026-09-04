# Security Policy for stale

`stale` is a fail-closed Rust guardrail library for autonomous AI agents.
It does not make economic decisions and it does not execute transactions.
This document describes what it trusts, what it does not trust, and how it fails.

## Summary

`stale` answers one question: is the on-chain state this agent is about to
act on safe enough under the caller's policy?

- `ALLOW` means the checks passed under the stated policy.
- `BLOCK` means do not proceed. Notify the human and stop.

`allow_execute` is permission state only. It is not an execution mechanism.
The invariant `allow_execute == (decision == Allow)` is upheld by the
constructors; values deserialized from untrusted JSON should be evaluated
via `GuardrailResult::is_allowed()` / `is_blocked()`, not the raw flag.

## Trust boundaries

- **Trusted:** the caller's policy values (`maxAgeSeconds`, thresholds,
  allowlists), the local clock only as an input to fail-closed math, and
  the Chainlink feed / contract addresses in the caller's allowlist.
- **Untrusted:** every byte from RPC responses (`answer`, `updatedAt`,
  `decimals`, round ids, bytecode, gas price, nonces), the RPC endpoint
  itself, URL strings, and any human or agent that calls a guard with a
  permissive policy. All are validated and fail closed.
- **Feed allowlist:** `check_price` only queries feeds in `src/feeds.rs`
  (`REGISTRY`, matched case-insensitively). Any valid-format address not in
  the registry fails closed (`unknown feed / not allowlisted`). Adding a
  feed is a registry change with tests and this doc updated.

## Chainlink assumptions

- Data Feeds: `latestRoundData()` returns
  `(uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt,
  uint80 answeredInRound)` and `decimals()` returns `uint8`.
- Never hardcode `decimals`. `stale` reads it on-chain and rejects values
  above 36.
- `updatedAt == 0` means no data yet → `BLOCK`.
- `answer <= 0` → `BLOCK`.
- Round completeness: `answeredInRound < roundId` means the round is
  incomplete / unanswered → `BLOCK`, even if `updatedAt` looks fresh.
  `startedAt` is not used for freshness (except the sequencer grace period).
- `updatedAt` values above `i64::MAX` are rejected as unrepresentable
  instead of being silently wrapped via `as` casts.
- Chain binding: L2 feeds additionally enforce sequencer liveness (below).
  `check_price` does not itself verify `eth_chainId` against the feed's
  registry chain — compose with `check_chain_id` when chain confusion is in
  scope.

## L2 sequencer policy

- Chains with a configured uptime feed (`get_sequencer_feed`) require
  `answer == 0` (up) **and** age past `GRACE_PERIOD_SECONDS` (3600s) since
  `startedAt`. Down, unknown status, incomplete round, missing data, future
  `startedAt`, and grace-period restarts all → `BLOCK`.
- Sequenced L2s with NO configured feed (Blast, Linea, Arbitrum Nova)
  BLOCK loudly (`is_unconfigured_sequenced_l2`) — never a silent pass.
  Chains without a centralized sequencer (mainnet, Polygon PoS) skip.
- Transport / decode failure → `BLOCK` (fail closed).

## RPC assumptions

- Transport: `HttpRpcClient` (`reqwest` + rustls, 10s timeout). No wallet,
  no signing, no `eth_sendTransaction` anywhere in the crate.
- **HTTPS mandatory** except loopback (`localhost`, `127.0.0.0/8`, `::1`),
  matched on the *parsed host* — never by string prefix — so
  `http://localhost.evil.com` is rejected.
- **Redirects are never followed** (`Policy::none`): any `3xx` response is
  refused and maps to `BLOCK`. Per-hop re-validation is unnecessary
  because hops never happen — the SSRF allowlist cannot be bypassed by a
  307 to a non-allowlisted host.
- Responses are capped at 1 MiB before JSON parsing (OOM bound against
  malicious endpoints).
- The configured URL (which may embed an API key) is redacted from every
  surfaced error, including credential and query fragments.
- Any RPC failure, malformed response, or unexpected shape → `BLOCK`.
  `stale` never substitutes cached data and never turns an operational
  failure into `ALLOW`.
- Hex parsing accepts `0x`/`0X`/bare; `eth_getCode` empty / zero-padded /
  malformed code is treated as EOA → `BLOCK`.

## Clock assumptions

- `nowSeconds` defaults to `chrono::Utc::now()`. A future `updatedAt`
  (`age < 0`) → `BLOCK` as `not-yet-valid`, covering local-clock-behind-chain
  skew. Negative caller clocks → `BLOCK`.
- No NTP sync is assumed. Pass `now_seconds` explicitly when you run your
  own time source.

## Precision and overflow

- `quote_from_feed` (`priceUsd` / `quoteUsd`) is `f64` **display math, not
  settlement math**: `answer as f64` rounds beyond 2⁵³. For 8-decimal feeds
  at current prices this is cent-exact; for settlement, keep integers and
  use `calculate_min_amount_out`, which is pure checked-`u128` math.
- `calculate_min_amount_out` rejects `amount_in == 0`, 100% slippage, dust
  that truncates to zero output, exponent diffs above 38, and every
  overflow (`Err` → caller emits `BLOCK`).
- Release **and** dev profiles enable `overflow-checks = true`; arithmetic
  uses `checked_*` throughout.
- `uint256` values are decoded to `u128`. On-chain values above `u128::MAX`
  (including literal `type(uint256).max` approvals) fail closed at decode
  time — the safe direction, surfaced as a decode-failure `BLOCK`. Full
  `U256` support is tracked work; see `src/allowance.rs` docs.
- `int256` values decode to `i128` with strict two's-complement validation;
  out-of-range values → `BLOCK`.

## Guard-specific caveats

- `check_paused`: a revert is read as "no `paused()` interface" → `ALLOW`.
  A malicious contract could `revert` to force that path, so compose with
  `AddressBook` + `check_is_contract` for unknown targets.
- `check_token_tax`: success proves *transferability*, not fair taxation.
  Fee-on-transfer / sell-tax tokens can pass; measure tax separately.
- `check_pool_v2` / `check_pool_v3`: spot depth at `latest` is flash-loan
  manipulable and unauthenticated. Require allowlisted pools + TWAP/deviation.
- `check_price_deviation`: checks round *completeness*, not *freshness*.
  Pair with an explicit `maxAge` policy.
- `check_nonce`: requires exact equality; ahead *or* behind → `BLOCK`.
- `simulate_tx` / `check_allowance` / `check_balance`: check-then-act
  advisories. State can move before broadcast — bind with deadlines,
  slippage limits, and private mempools.
- `RateLimiter` / `SpendingCap`: in-memory, `&mut self` (share via `Mutex`
  across tasks), reset on restart, capped at 100k entries. Use atomic
  `try_acquire` / `try_spend`; manual `check` + `record` is TOCTOU.
- `GuardPipeline`: per-guard timeout (default 15s) + panic isolation turn
  hangs and panics into `BLOCK`s attributed to that guard. Max 64 guards.
- `AuditLogger`: bounded FIFO (`VecDeque`); `Some(0)` capacity coerces to 1.
  Callbacks are panic-isolated but synchronous — keep them non-blocking.
- Secret scrub (`audit::scrub_secrets`) is ASCII case-insensitive and covers
  `key=VALUE` forms plus `Bearer`/`Basic` tokens; it is applied both in
  `AuditLogger::record` and in `GuardPipeline` reports, so neither sink
  becomes a secret store.
- `check_mev_rpc`: host-allowlisted private builders, `https` required
  except loopback dev endpoints.

## MCP risks

- `stale-mcp` is stdio line-delimited JSON-RPC only. No HTTP, no SSE.
- Tools: `stale_isStale` (pure), `stale_quote` (math only), `stale_check`
  (full `check_price`, no wallet). All delegate to the same core.
- **BLOCK contract:** a `BLOCK` verdict is `result.content[0].text` JSON
  **with `isError: true`**. Agents must treat `isError == true` as "do not
  execute" — checking only the absence of a protocol `error` is a fail-open
  integration bug.
- `maxAgeSeconds` is required with no silent default; a missing value is an
  `isError` rejection, not `maxAge 0`.
- Each request line is capped at 1 MiB.
- Read-only: no private key, no signing, no broadcast.

## CLI risks

- `stale check --rpc … --max-age … [--feed …] [--amount …] [--json]`.
  Human mode prints decision/reason/price; `--json` prints the result object.
  Exit `0` on `ALLOW`, `1` on `BLOCK` or usage error.
- `stale is-stale --updated-at … --max-age … [--now …]`, `stale quote`,
  `stale feeds` are offline except `check`.

## Dependency and supply-chain risks

- Runtime: `tokio`, `serde`/`serde_json`, `reqwest` (rustls-tls only, no
  default features), `clap` (derive), `hex`, `async-trait`, `chrono`, `url`.
- Avoid adding network or crypto dependencies in guard paths without review.
  `Cargo.lock` handling follows the repo's library convention;
  `cargo package` must stay clean (only `src/`, `tests/`, `examples/`,
  `Cargo.toml`, `README.md`, `LICENSE` per `include`).

## Fail-closed semantics

Every public entry — `is_stale`, `quote_from_feed`, `check_price`,
`check_prices`, all guards, the CLI, `stale-mcp` tools, and
`GuardPipeline::run` — is fail-closed. The only way to get `ALLOW` and
`allow_execute: true` is to pass *every* check. Any doubt → `BLOCK`.

Never claim stronger guarantees than the implementation. A new check needs
a reason, an implementation, tests, docs, and an update to this file.

## Reporting a vulnerability

Do not open a public issue for a sensitive security report. Instead, open
a draft PR with a failing test that reproduces the `ALLOW` on bad data, or
contact the maintainer via https://ramprasadgoud.dev and describe the
unsafe `ALLOW` path with steps to reproduce.

## License

MIT — see `LICENSE`. This security documentation is part of the project and
is licensed under the same terms.
