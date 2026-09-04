# stale

`stale` is a fail-closed DeFi security guardrail library for autonomous AI agents, written in Rust.

Before an agent signs and broadcasts a transaction, `stale` runs pre-flight checks against on-chain state and oracle feeds. If an oracle is stale, an L2 sequencer is down, a pool lacks liquidity, or an approval is unbounded, execution blocks immediately.

[![Crates.io](https://img.shields.io/crates/v/stale.svg)](https://crates.io/crates/stale)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/Ramprasad4121/stale/actions/workflows/ci.yml/badge.svg)](https://github.com/Ramprasad4121/stale/actions)

---

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
stale = "2.0.0"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Or install the CLI and MCP server:

```bash
cargo install stale
```

---

## Quickstart

### Composable Pre-Flight Pipeline

```rust
use stale::prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = HttpRpcClient::new("https://rpc.flashbots.net");

    // 1. Allowlist trusted contracts (strict: unknown → BLOCK)
    let mut allowlist = HashMap::new();
    allowlist.insert("UNISWAP_V3_ROUTER".into(), "0xE592427A0AEce92De3Edee1F18E0157C05861564".into());
    let book = AddressBook::new_strict(allowlist)?;

    // 2. Rate limiter, shared into the guard so every preflight acquires
    //    live. A limiter that is never consulted limits nothing.
    let limiter = Arc::new(Mutex::new(RateLimiter::new(10, 60)?)); // 10 tx / 60s

    // 3. Build pipeline
    let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);

    pipeline.add("mev_rpc", || async {
        check_mev_rpc("https://rpc.flashbots.net")
    });

    let router = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    let addr_check = book.check(router);
    pipeline.add("address_book", move || {
        let res = addr_check.clone();
        async move { res }
    });

    let limiter_guard = limiter.clone();
    pipeline.add("rate_limit", move || {
        let limiter = limiter_guard.clone();
        async move {
            limiter
                .lock()
                .map(|mut l| l.try_acquire())
                .unwrap_or_else(|_| {
                    stale::types::GuardrailResult::block(
                        "rate limiter lock poisoned — BLOCK (fail closed)",
                    )
                })
        }
    });

    let client = rpc.clone();
    pipeline.add("gas_price", move || {
        let c = client.clone();
        async move { check_gas_price(&c, 40).await } // max 40 Gwei
    });

    let feed_client = rpc.clone();
    pipeline.add("oracle_freshness", move || {
        let c = feed_client.clone();
        async move {
            let r = check_price(
                &c,
                CheckPriceInput {
                    feed: DEFAULT_FEED, // ETH/USD
                    max_age_seconds: 60,
                    amount_eth: None,
                    now_seconds: None,
                },
            )
            .await;
            if r.decision == Decision::Allow {
                GuardrailResult::allow(r.reason)
            } else {
                GuardrailResult::block(r.reason)
            }
        }
    });

    // 4. Evaluate (no unwraps: every field is optional on some path)
    let report = pipeline.run().await;
    if report.decision == Decision::Block {
        eprintln!(
            "Blocked by {}: {}",
            report.blocked_by.as_deref().unwrap_or("unknown"),
            report.reason
        );
        std::process::exit(1);
    }

    println!("All checks passed ({:.2}ms). Proceeding to execute.", report.duration_ms);
    Ok(())
}
```

### Checking a Price Feed Directly

```rust
use stale::prelude::*;

#[tokio::main]
async fn main() {
    let rpc = HttpRpcClient::new("https://ethereum-rpc.publicnode.com");

    let result = check_price(
        &rpc,
        CheckPriceInput {
            feed: DEFAULT_FEED, // ETH/USD
            max_age_seconds: 60,
            amount_eth: Some(1.0),
            now_seconds: None,
        },
    )
    .await;

    if result.decision == Decision::Allow {
        match result.price_usd {
            Some(price) => println!("Price: ${:.2}", price),
            None => eprintln!("Guard allowed without a price (unexpected)"),
        }
    } else {
        eprintln!("Price check blocked: {}", result.reason);
    }
}
```

---

## For AI Agents

Building an agent on `stale`? Read **[PROMPT.md](PROMPT.md)**: the copy-paste wiring block, the 7 rules your agent must obey (`BLOCK` handling, live governor acquisition, exit codes, MCP `isError`), and the docs map. Start there, not here.

---

## Available Guardrails

### Oracles & Feeds
- **`check_price`**: Validates Chainlink Data Feed round completeness, age, and pricing.
- **`check_price_deviation`**: Compares two independent price feeds to detect single-oracle manipulation (library only — no CLI subcommand or MCP tool; integrate via the Rust API).
- **`is_stale`**: Pure, offline staleness and clock-skew math without network calls.
- **`lookup_feed`**: Static registry (`src/feeds.rs`, 15 feeds across Ethereum, Arbitrum, Optimism, Base, Polygon, zkSync, Metis, Mantle, and Scroll).

### Network & Infrastructure
- **`check_gas_price`**: BLOCKs when `eth_gasPrice` exceeds policy (integer-wei compare, `f64` display-only); RPC failure or `0` policy → BLOCK.
- **`check_gas_price_1559`**: EIP-1559 circuit breaker enforcing `baseFeePerGas` and `maxPriorityFeePerGas` independently; missing fee / pre-1559 chain → BLOCK.
- **`check_sequencer`**: Validates L2 sequencer uptime (Arbitrum, OP, Base, Scroll, zkSync, Metis, Mantle) and enforces a 3600s restart grace period.
- **`check_mev_rpc`**: Ensures the RPC endpoint routes to private builders (Flashbots, MEVBlocker, Titan, Beaver, BloxRoute, Eden) rather than public mempools.
- **`check_rpc_sync`**: Checks that the RPC node's latest block timestamp is within acceptable drift of current time.
- **`check_chain_id`**: Guards against cross-chain configuration mistakes.
- **`check_nonce`**: Detects nonce desynchronization before broadcast.

### Execution & Liquidity
- **`calculate_min_amount_out`**: Derives integer slippage boundaries from oracle prices and basis points.
- **`check_pool_v2` / `check_pool_v3`**: Checks Uniswap V2 reserve depths and Uniswap V3 active in-range liquidity.
- **`check_deadline`**: Verifies transaction deadlines are neither expired nor excessively far in the future.
- **`simulate_tx`**: Runs pre-flight `eth_call` simulation; blocks if execution reverts.

### Assets & Permissions
- **`check_approval`**: Rejects infinite approvals (`type(uint256).max` or dangerously large values).
- **`check_allowance`**: Confirms sufficient token allowance on-chain before initiating swaps.
- **`check_balance`**: Confirms sufficient Native ETH or ERC20 balance before sending transactions.
- **`check_token_tax`**: Simulates ERC20 transfers to detect honeypots and unadvertised fees.
- **`check_paused`**: Checks whether target contracts have triggered emergency pause states.

### Compliance & Access Control
- **`AddressBook`**: In-memory allowlist of verified contract targets.
- **`RateLimiter`**: Rolling sliding-window transaction frequency governor.
- **`SpendingCap`**: Rolling sliding-window cumulative spending cap.
- **`AuditLogger`**: Structured JSON logging of every ALLOW and BLOCK event.
- **`check_is_contract`**: Verifies target address has bytecode (`eth_getCode`), blocking EOA phishing traps. Proves has-bytecode only — proxies, upgradeables, and honeypots pass; compose with `AddressBook` for unknown targets.
- **`check_sanctioned`**: Queries the Chainlink OFAC Sanctions Oracle.

---

## Model Context Protocol (MCP) Server

`stale-mcp` exposes guardrails to AI agent environments (Claude Desktop, Cursor, ElizaOS) over stdio JSON-RPC:

```json
{
  "mcpServers": {
    "stale": {
      "command": "stale-mcp"
    }
  }
}
```

Exposed tools:
- `stale_check`: Full on-chain Chainlink feed query, staleness verification, and quotation.
- `stale_isStale`: Pure offline staleness check.
- `stale_quote`: Decimal-scaled price quotation.

---

## CLI

```bash
# Check price feed freshness
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 --amount 1.0

# Emit JSON for shell scripts / CI
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 --json

# Restrict RPC egress (SSRF guard; unset = unrestricted)
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 \
  --allowed-rpc-hosts ethereum-rpc.publicnode.com

# Offline timestamp check (always JSON: decision/ageSeconds/reason)
stale is-stale --updated-at 1700000000 --max-age 60

# Price math from a raw answer (decimal or 0x hex; always JSON)
stale quote --answer 250000000000 --decimals 8 --amount 1.0

# Print supported feeds
stale feeds
```

Exit-code contract: `0` = ALLOW, `1` = BLOCK (or guard failure), `2` = usage/config error (missing flags, negative `--max-age`, unparseable `--answer`). Shapes differ per command: `check --json` emits the full `CheckPriceResult` (incl. `allowExecute`); `is-stale` emits `IsStaleResult` (`decision`/`ageSeconds`/`reason` only); `quote` emits `QuoteResult` (`priceUsd`/`quoteUsd`, no verdict — failures exit `1` with an `Error:` line on stderr).

Chainlink CRE simulation (TypeScript workflow mirroring `check_price`): see [`cre/README.md`](cre/README.md).

---

## Testing

All tests use a mocked RPC transport and run completely offline with no network dependencies:

```bash
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

---

## License

MIT
