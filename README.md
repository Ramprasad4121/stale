# 🛡️ stale

> **Fail-closed DeFi security guardrails for autonomous AI agents.**  
> Written in 100% pure Rust. Zero bloat. Sub-millisecond execution. Thread-safe.

[![Crates.io](https://img.shields.io/crates/v/stale.svg)](https://crates.io/crates/stale)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build & Test](https://github.com/Ramprasad4121/stale/actions/workflows/ci.yml/badge.svg)](https://github.com/Ramprasad4121/stale/actions)

Autonomous agents are naive to the dark forest of DeFi. Left unattended, agents execute trades with dangerous slippage, route transactions through public mempools where MEV bots sandwich them, approve infinite tokens (`MaxUint256`) to vulnerable contracts, or buy un-sellable honeypot tokens.

`stale` intercepts agent intents **before** on-chain submission. Every transaction must pass through a composable, fail-closed guardrail pipeline. If ANY condition deviates from safety, `stale` defaults to **`BLOCK`**.

---

## ⚡ The 10-Point Defense System

1. **Chainlink Oracle Staleness (`check_price`)**: Queries Chainlink Data Feeds directly and enforces maximum age boundaries. If the feed hasn't updated within your threshold, execution is blocked.
2. **L2 Sequencer Uptime (`check_sequencer`)**: Checks Arbitrum, Optimism, Base, Scroll, zkSync, Metis, and Mantle sequencer feeds with automatic 3600-second grace period enforcement.
3. **Multi-Oracle Price Deviation (`check_price_deviation`)**: Compares two independent oracles for the same pair. If prices deviate, flags potential flash-loan manipulation and blocks.
4. **DEX Liquidity Depth (`check_pool_v2`, `check_pool_v3`)**: Inspects Uniswap V2 reserves and Uniswap V3 in-range liquidity to prevent agents from trading in shallow or drained pools.
5. **Dynamic Slippage & MEV Bounds (`calculate_min_amount_out`)**: Calculates exact `minAmountOut` thresholds from oracle prices with basis-point slippage tolerances, eliminating sandwich attacks.
6. **Strict Approvals & Allowances (`check_approval`, `check_allowance`)**: Strictly forbids infinite approvals (`type(uint256).max`), enforcing exact amount approvals and validating on-chain allowances.
7. **Gas Price Circuit Breaker (`check_gas_price`)**: Halts execution when network base fees spike during congestion, preserving the agent's treasury.
8. **Token Tax & Honeypot Detection (`check_token_tax`)**: Simulates token transfers to dead addresses; blocks execution if transfers revert or charge hidden taxes.
9. **Pre-flight Simulation & Solvency (`simulate_tx`, `check_balance`)**: Executes `eth_call` simulations and checks balances before broadcasting to prevent wasted gas on reverted transactions.
10. **Compliance, Network & Access Control (`check_sanctioned`, `check_mev_rpc`, `check_is_contract`, `AddressBook`, `RateLimiter`, `SpendingCap`)**: 
    - Queries the Chainlink OFAC Sanctions Oracle.
    - Enforces private/MEV-protected RPCs (Flashbots, MEVBlocker, Titan, Beaver).
    - Checks contract bytecode to prevent EOA phishing.
    - Enforces sliding-window transaction frequency limits and cumulative spending caps.
    - Restricts interactions to an immutable `AddressBook` allowlist.

---

## 📦 Installation

Add `stale` to your `Cargo.toml`:

```toml
[dependencies]
stale = "1.0.0"
tokio = { version = "1", features = ["full"] }
```

Or via `cargo add`:

```bash
cargo add stale
```

To install the CLI and MCP Server:

```bash
cargo install stale
```

---

## 🚀 Quick Start

### 1. The Composable Pre-Flight Pipeline

Chain multiple guardrails with fail-fast execution and structured compliance logging:

```rust
use stale::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = HttpRpcClient::new("https://rpc.flashbots.net");
    let mut audit = AuditLogger::default();

    // 1. Setup trusted contracts
    let mut allowlist = HashMap::new();
    allowlist.insert(
        "UNISWAP_ROUTER".to_string(),
        "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
    );
    let book = AddressBook::new(allowlist, true)?;

    // 2. Build pipeline
    let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, Some(audit.clone()));

    // Guard 1: Verify MEV-protected RPC
    pipeline.add("mev_protection", || async {
        check_mev_rpc("https://rpc.flashbots.net")
    });

    // Guard 2: Gas circuit breaker (< 40 Gwei)
    let rpc_clone = rpc.clone();
    pipeline.add("gas_breaker", move || {
        let client = rpc_clone.clone();
        async move { check_gas_price(&client, 40).await }
    });

    // Guard 3: Strict allowance check
    let rpc_clone = rpc.clone();
    pipeline.add("allowance", move || {
        let client = rpc_clone.clone();
        async move {
            check_allowance(
                &client,
                "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
                "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // Agent
                "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Router
                1_000_000_000,
            ).await
        }
    });

    // 3. Run all pre-flight checks
    let report = pipeline.run().await;

    if report.decision == Decision::Block {
        panic!("Preflight blocked by {}: {}", report.blocked_by.unwrap(), report.reason);
    }

    println!("All guards passed in {:.2}ms! Safe to execute.", report.duration_ms);
    Ok(())
}
```

### 2. Standalone Price Guardrail

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
            amount_eth: Some(1.5),
            now_seconds: None,
        },
    )
    .await;

    println!("Decision: {}", result.decision);
    if result.decision == Decision::Allow {
        println!("Price: ${:.2}", result.price_usd.unwrap());
        println!("Quote: ${:.2}", result.quote_usd.unwrap());
    }
}
```

---

## 🛠️ CLI Tool

`stale` includes a standalone command-line interface for terminal monitoring, shell scripts, and CI/CD:

```bash
# Check price feed freshness
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 --amount 1.0

# Output clean JSON for scripts
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 --json

# Offline staleness calculation
stale is-stale --updated-at 1700000000 --max-age 60

# List built-in feed registry
stale feeds
```

---

## 🤖 Model Context Protocol (MCP) Server

`stale` ships with a native Model Context Protocol server (`stale-mcp`), allowing AI agents (Claude Desktop, Cursor, Eliza, LangChain) to use `stale` guardrails as native agent tools over stdio:

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "stale": {
      "command": "stale-mcp"
    }
  }
}
```

Exposed MCP tools:
- `stale_isStale`: Pure, offline staleness verification.
- `stale_quote`: Precise quote math with arbitrary decimal scaling.
- `stale_check`: End-to-end live on-chain guardrail query.

---

## 🧪 Testing & Verification

The test suite runs 100% offline with zero external network dependencies via mocked RPC clients:

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
cargo package
```

---

## 📄 License

MIT © [Ramprasad Goud](https://github.com/Ramprasad4121)
