<div align="center">

# 🛡️ stale

**The Fail-Closed Execution Firewall for Autonomous On-Chain AI Agents.**  
*Built in 100% Safe, Zero-Cost-Abstraction Rust.*

[![Crates.io](https://img.shields.io/crates/v/stale.svg?style=flat-square&color=orange)](https://crates.io/crates/stale)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](https://opensource.org/licenses/MIT)
[![Build Status](https://img.shields.io/github/actions/workflow/status/Ramprasad4121/stale/ci.yml?branch=main&style=flat-square&label=CI)](https://github.com/Ramprasad4121/stale/actions)
[![MCP Ready](https://img.shields.io/badge/MCP-Native%20Stdio-green.svg?style=flat-square)](https://modelcontextprotocol.io)
[![Benchmarks](https://img.shields.io/badge/Pipeline%20Latency-0.16ms-purple.svg?style=flat-square)](#-performance--benchmarks)

</div>

---

## ⚡ The Dark Forest Problem

Autonomous LLM agents transacting on-chain are naive execution engines operating in an adversarial environment. Left unguarded, agents routinely fall prey to critical financial vectors:
- **Oracle Manipulation & Flash Loans**: Acting on single-oracle prices skewed within a single transaction block.
- **MEV Sandwich Attacks**: Emitting trades with zero or loose slippage into the public mempool.
- **L2 Sequencer Outages**: Consuming stale oracle states while Arbitrum or Optimism sequencers are down.
- **Honeypot Tokens**: Swapping into ERC20 tokens with un-transferable code or 99% malicious transfer taxes.
- **Infinite Allowance Exploits**: Signing `approve(spender, type(uint256).max)` to untrusted or vulnerable routers.
- **Gas War Exhaustion**: Looping failing transactions during base-fee spikes, draining treasury balances in minutes.

**`stale` is the uncompromising, fail-closed firewall.**  
Every transaction intent must survive a battery of deterministic on-chain and offline guardrails. If a single check deviates, encounters an RPC timeout, or receives malformed data, **`stale` defaults to `BLOCK`**.

```
[ AI Agent Intent ] 
        │
        ▼
┌─────────────────────────────────────────────────────────────┐
│                 stale Pre-Flight Firewall                   │
│                                                             │
│  ├─ 1. Access Control      (AddressBook, MEV RPC, Nonce)    │
│  ├─ 2. Oracle Freshness    (Chainlink, Sequencer, Variance) │
│  ├─ 3. Pool Depth & MEV    (Uniswap V2/V3, Slippage Bps)    │
│  ├─ 4. Contract Security   (Bytecode Check, Paused State)   │
│  ├─ 5. Asset Safety        (Honeypot Simulation, Approval)  │
│  └─ 6. Treasury Guard      (RateLimiter, SpendingCap, Gas)  │
└──────────────────────────────┬──────────────────────────────┘
                               │
            ┌──────────────────┴──────────────────┐
            ▼                                     ▼
      [  BLOCK  ]                           [  ALLOW  ]
(Transaction Terminated)            (Safe On-Chain Execution)
```

---

## 🏛️ The 10-Point Defense System (20+ Native Guards)

| Category | Guardrail | Defense Mechanism | Invariant / Failure Mode |
|:---|:---|:---|:---|
| **1. Oracle Health** | `check_price` | Queries official Chainlink Data Feeds directly | `age > max_age` or `updated_at == 0` $\rightarrow$ **BLOCK** |
| | `check_price_deviation` | Compares two independent oracles for identical pair | `deviation > max_deviation_percent` $\rightarrow$ **BLOCK** |
| | `is_stale` | Offline mathematical staleness and clock-skew validator | `updated_at > now` (future timestamp) $\rightarrow$ **BLOCK** |
| **2. L2 Infrastructure**| `check_sequencer` | Arbitrum, OP, Base, Scroll, zkSync, Metis, Mantle | Sequencer DOWN or `grace_period < 3600s` $\rightarrow$ **BLOCK** |
| **3. Liquidity & Depth**| `check_pool_v2` | Inspects Uniswap V2 reserve reserves | `reserve0 < min_reserve0` $\rightarrow$ **BLOCK** |
| | `check_pool_v3` | Validates Uniswap V3 in-range active liquidity | `liquidity < min_liquidity` $\rightarrow$ **BLOCK** |
| **4. MEV Protection** | `calculate_min_amount_out`| Computes exact integer `minAmountOut` via oracle | Strict basis-point slippage tolerance $\rightarrow$ **BLOCK** |
| | `check_mev_rpc` | Validates private RPCs (Flashbots, Titan, Beaver) | Public mempool endpoint detected $\rightarrow$ **BLOCK** |
| **5. Token Security** | `check_token_tax` | Simulates ERC20 transfer to dead address | Transfer reverts or fees detected $\rightarrow$ **BLOCK** |
| | `check_approval` | Prevents runaway permissions | `amount == MaxUint256` (infinite approval) $\rightarrow$ **BLOCK** |
| | `check_allowance` | Validates on-chain allowance before swap execution | `allowance < required_amount` $\rightarrow$ **BLOCK** |
| **6. Solvency & Gas** | `check_balance` | Pre-flight Native ETH or ERC20 balance verification | `balance < required_amount` $\rightarrow$ **BLOCK** |
| | `check_gas_price` | Network basefee circuit breaker | `gas_price > max_gas_price_gwei` $\rightarrow$ **BLOCK** |
| **7. Protocol Status** | `check_paused` | Queries target contract `paused()` implementation | Protocol in emergency pause state $\rightarrow$ **BLOCK** |
| | `simulate_tx` | Executes native pre-flight `eth_call` | Execution reverts for any reason $\rightarrow$ **BLOCK** |
| **8. Anti-Phishing** | `check_is_contract` | Checks target address bytecode via `eth_getCode` | Target is an EOA wallet (phishing trap) $\rightarrow$ **BLOCK** |
| | `check_sanctioned` | Queries Chainlink OFAC Sanctions Oracle on-chain | Address flagged on sanctions registry $\rightarrow$ **BLOCK** |
| **9. Network Sync** | `check_rpc_sync` | Validates node sync and block timestamp freshness | Node is lagging behind wall clock $\rightarrow$ **BLOCK** |
| | `check_chain_id` | Verifies RPC network ID matches expected chain | Cross-chain replay misconfiguration $\rightarrow$ **BLOCK** |
| | `check_nonce` | Verifies sender account nonce against expected state | Nonce desync / pending TX race $\rightarrow$ **BLOCK** |
| **10. Treasury Limits** | `RateLimiter` | Sliding-window transaction frequency governor | Limit exceeded within rolling window $\rightarrow$ **BLOCK** |
| | `SpendingCap` | Sliding-window cumulative treasury value cap | Cumulative value exceeds cap $\rightarrow$ **BLOCK** |
| | `AddressBook` | Immutable allowlist of verified contract addresses | Unknown address not in allowlist $\rightarrow$ **BLOCK** |
| | `check_deadline` | Swap intent deadline validity validator | Deadline expired or excessively far $\rightarrow$ **BLOCK** |

---

## ⏱️ Performance & Benchmarks

In autonomous trading, CPU latency must never be the bottleneck. Built in Rust with zero memory allocation hot paths:

```
Guardrail Pipeline Execution (6 Guards: Allowlist, Deadline, Slippage, MEV, RateLimit, SpendingCap):
  Total Pipeline Execution Time : 0.16 ms (160 microseconds)
  Memory Footprint              : < 2.0 MB
  Async Concurrency             : Native Tokio multi-threaded work-stealing runtime
```

---

## 📦 Installation

Add `stale` to your `Cargo.toml`:

```toml
[dependencies]
stale = "1.0.0"
tokio = { version = "1", features = ["full"] }
```

Or via the command line:

```bash
cargo add stale
```

Install the standalone CLI and MCP Server:

```bash
cargo install stale
```

---

## 🛠️ Usage Guide

### 1. Composable Pre-Flight Pipeline

The recommended pattern for autonomous agents: assemble guards into a single `GuardPipeline` with fail-fast execution and structured audit logging.

```rust
use stale::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc = HttpRpcClient::new("https://rpc.flashbots.net");
    let audit = AuditLogger::default();

    // 1. Immutable Contract Allowlist
    let mut allowlist = HashMap::new();
    allowlist.insert(
        "UNISWAP_V3_ROUTER".to_string(),
        "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
    );
    let book = AddressBook::new(allowlist, true)?;

    // 2. Sliding Window Circuit Breakers
    let mut rate_limiter = RateLimiter::new(10, 60)?; // max 10 tx/min
    let mut spending_cap = SpendingCap::new(5_000_000_000_000_000_000, 3600)?; // max 5 ETH/hour

    // 3. Assemble Pipeline
    let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, Some(audit));

    // Guard: Enforce MEV-protected RPC
    pipeline.add("mev_protection", || async {
        check_mev_rpc("https://rpc.flashbots.net")
    });

    // Guard: Verify target router is allowlisted
    let router = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    let check = book.check(router);
    pipeline.add("address_book", move || {
        let res = check.clone();
        async move { res }
    });

    // Guard: Gas Price Circuit Breaker (< 40 Gwei)
    let client = rpc.clone();
    pipeline.add("gas_breaker", move || {
        let c = client.clone();
        async move { check_gas_price(&c, 40).await }
    });

    // Guard: Rate Limit
    let rate_check = rate_limiter.check();
    pipeline.add("rate_limiter", move || {
        let res = rate_check.clone();
        async move { res }
    });

    // 4. Run Pre-Flight Check
    let report = pipeline.run().await;

    if report.decision == Decision::Block {
        eprintln!("🚨 EXECUTION BLOCKED by [{}]: {}", report.blocked_by.unwrap(), report.reason);
        std::process::exit(1);
    }

    println!("✅ All guards passed in {:.2}ms. Safe to broadcast.", report.duration_ms);
    Ok(())
}
```

### 2. Standalone Price Feed Verification

```rust
use stale::prelude::*;

#[tokio::main]
async fn main() {
    let rpc = HttpRpcClient::new("https://ethereum-rpc.publicnode.com");

    let result = check_price(
        &rpc,
        CheckPriceInput {
            feed: DEFAULT_FEED, // ETH/USD Mainnet
            max_age_seconds: 60,
            amount_eth: Some(2.5),
            now_seconds: None,
        },
    )
    .await;

    match result.decision {
        Decision::Allow => {
            println!("Price: ${:.2}", result.price_usd.unwrap());
            println!("Quote: ${:.2}", result.quote_usd.unwrap());
        }
        Decision::Block => {
            panic!("Price check failed: {}", result.reason);
        }
    }
}
```

---

## 🤖 Model Context Protocol (MCP) Server

`stale` ships with `stale-mcp`, an enterprise-grade Model Context Protocol server communicating over stdio. It enables any LLM agent (Claude Desktop, Cursor, ElizaOS, Goose, LangChain) to execute native security checks before crafting transactions.

### Claude Desktop / Cursor Configuration

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

### Exposed Tools:
* **`stale_check`**: Multi-step on-chain query (Chainlink `latestRoundData`, decimals, sequencer uptime, and quotation).
* **`stale_isStale`**: Pure offline mathematical staleness and clock-skew calculation.
* **`stale_quote`**: Arbitrary-precision decimal scaling and fiat value computation.

---

## 🖥️ Command-Line Interface (`stale`)

The `stale` CLI allows DevSecOps engineers and CI/CD pipelines to monitor feeds and execute guardrail assertions directly from shell scripts:

```bash
# Query live feed freshness
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 --amount 1.0

# Emit pure JSON for automated scripts
stale check --rpc https://ethereum-rpc.publicnode.com --max-age 60 --json

# Offline timestamp validation
stale is-stale --updated-at 1700000000 --max-age 60

# Inspect built-in multi-chain feed registry
stale feeds
```

---

## 🧪 Testing & Verification

The test suite is completely decoupled from live network conditions. Using a zero-network mock transport, 70 unit and integration tests execute in **0.01 seconds**:

```bash
# Run unit & integration test suite
cargo test --all-targets

# Verify strict clippy conformance
cargo clippy --all-targets -- -D warnings

# Verify formatting
cargo fmt --all -- --check

# Verify crate packaging
cargo package
```

---

## 🌐 Supported Networks & Feeds

`stale` maintains native, static registries for Chainlink oracles and L2 Sequencer Uptime feeds across:
- **Ethereum Mainnet** (Chain ID: `1`)
- **Arbitrum One** (Chain ID: `42161`)
- **Optimism** (Chain ID: `10`)
- **Base** (Chain ID: `8453`)
- **Polygon PoS** (Chain ID: `137`)
- **Scroll** (Chain ID: `534352`)
- **zkSync Era** (Chain ID: `324`)
- **Metis** (Chain ID: `1088`)
- **Mantle** (Chain ID: `5000`)

---

## 📜 License

Distributed under the **MIT License**. See `LICENSE` for details.

Developed with fail-closed rigor by **[Ramprasad Goud](https://github.com/Ramprasad4121)**.
