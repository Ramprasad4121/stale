# PROMPT.md — give this to your agent

Copy-paste block for wiring `stale` (fail-closed DeFi guardrails, Rust) into an autonomous agent. This file is the single source; `README.md` only points here.

---

`stale` answers one question before you sign or broadcast anything: **is the on-chain state safe enough under my policy?** It returns `ALLOW` (proceed) or `BLOCK` (stop, notify the human, do not retry silently). Any doubt — stale feed, RPC failure, decode failure, misconfiguration — is a `BLOCK`. `stale` never signs, never sends transactions, never holds keys.

**Wire it (Rust):**

```rust
use stale::prelude::*;

// FailFast stops at the first BLOCK; RunAll evaluates everything.
let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);
let rpc = HttpRpcClient::new("https://ethereum-rpc.publicnode.com");

// Address allowlist first: unknown contracts BLOCK.
let book = AddressBook::new_strict(my_allowlist)?;
let target = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
let check = book.check(target);
pipeline.add("address_book", move || {
    let res = check.clone();
    async move { res }
});

// Then price freshness (production max-age: 60s), gas, sequencer…
let report = pipeline.run().await;
if report.decision == Decision::Block {
    // BLOCK means DO NOT EXECUTE. Notify the human with report.reason.
    return Err(report.reason.into());
}
```

**Rules you must obey:**

1. `BLOCK` is final for this preflight. Do not re-run with looser policy to get an `ALLOW`.
2. Spend-affecting governors (`RateLimiter`, `SpendingCap`) must be called **live inside the guard** via `try_acquire` / `try_spend` (shared `Arc<Mutex<…>>`). Never snapshot `check()` outside and replay it — the limiter would never trip.
3. CLI exit codes: `0` = ALLOW, `1` = BLOCK, `2` = usage error. With `--json`, parse the result object; without it, read the printed verdict.
4. MCP (`stale-mcp`, stdio JSON-RPC): a `BLOCK` arrives as result text **with `isError: true`**. Treat `isError == true` as "do not execute" — the absence of a protocol `error` field is NOT an allow.
5. `check_price` only queries feeds in the `src/feeds.rs` registry — unknown addresses BLOCK. It does not verify chain id; compose with `check_chain_id` when chain confusion is in scope.
6. New transports must fail closed: unimplemented RPC methods return `Err`, and every `Err` becomes a `BLOCK`. Never substitute cached data on failure.
7. Production freshness policy is `max_age_seconds: 60`. Demos may use larger windows; say so explicitly when you do.
8. MCP strictness: `stale_isStale` requires explicit `nowSeconds` (no clock default — unlike the CLI); every `content[0].text` is JSON-parseable, including bad-argument rejections. `allowedRpcHosts` must be an array of bare hosts (no ports — ports never match); a wrong type BLOCKs.
9. Sequencer coverage follows the feed registry: chains without registry feeds (Blast, Linea, Nova) cannot reach the sequencer check via `check_price` — call `check_sequencer(chain_id)` directly for those. `PipelineResult.guardsSkipped` is present only when `FailFast` skips guards — treat it as optional.

**Docs map:** `README.md` (install, guardrails, CLI), `SECURITY.md` (trust model, caveats per guard), `CONTRIBUTING.md` (PR flow, test commands), `cre/README.md` (Chainlink CRE simulation).

**Verify your wiring:** `cargo test --all-targets`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --all -- --check`. Tests use `MockRpcClient` — no live RPC, no wallets.
