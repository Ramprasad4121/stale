# `stale` Rust Architecture & Threat Model

`stale` is built from first principles in 100% safe, modern Rust to serve as the uncompromised fail-closed security backbone for autonomous onchain AI agents.

## Core Philosophy: Strict Fail-Closed

In autonomous DeFi systems, any error, timeout, RPC failure, or malformed payload MUST be treated as an existential security event.

`stale` adheres to strict mathematical rules:
1. If an RPC call returns an error, times out, or fails to parse -> `BLOCK`.
2. If an oracle price is zero, negative, unparseable, or older than `max_age_seconds` -> `BLOCK`.
3. If an L2 sequencer reports down or is within its 3600-second restart grace period -> `BLOCK`.
4. If a target contract address is an EOA (no bytecode) -> `BLOCK`.
5. If an approval requests infinite allowance (`type(uint256).max`) -> `BLOCK`.
6. If an asset transfer simulation reverts -> `BLOCK` (flagged as honeypot).

## Module Layout

```
src/
├── lib.rs          # Root exports and prelude
├── types.rs        # Decision, GuardrailResult
├── is_stale.rs     # Pure offline staleness logic
├── quote.rs        # Arbitrary-precision decimal quote math
├── feeds.rs        # Static Chainlink Feed Registry (Mainnet, L2s)
├── rpc.rs          # JSON-RPC 2.0 client & EvmRpcClient trait
├── mock.rs         # MockRpcClient for zero-network testing
├── abi.rs          # Zero-dependency ABI encoding/decoding
├── sequencer.rs    # L2 Sequencer Uptime Feed guardrail
├── dex.rs          # Uniswap V2/V3 liquidity depth guards
├── slippage.rs     # Dynamic slippage calculation
├── allowance.rs    # Strict approvals & on-chain allowance checks
├── gas.rs          # Network gas price spike circuit breaker
├── solvency.rs     # Native & ERC20 balance verification
├── pausable.rs     # Protocol pause state guardrail
├── simulate.rs     # Pre-flight eth_call transaction simulation
├── sanctions.rs    # Chainlink OFAC Sanctions Oracle verification
├── mev.rs          # MEV-protected private RPC enforcement
├── contract.rs     # Bytecode verification against EOA phishing
├── network.rs      # RPC sync, chain ID, and nonce desync checks
├── ratelimit.rs    # RateLimiter & SpendingCap sliding windows
├── audit.rs        # Structured JSON compliance logger (AuditEntry lives here)
├── pipeline.rs     # Composable GuardPipeline with fail-fast execution
├── honeypot.rs     # Token transfer tax / honeypot detection
├── deviation.rs    # Multi-oracle price deviation guard
├── deadline.rs     # Swap intent deadline validation
├── addressbook.rs  # Strict contract allowlist
└── check.rs        # End-to-end Chainlink price feed checking

bin/
├── stale.rs        # CLI tool
└── stale_mcp.rs    # Model Context Protocol (MCP) server
```

## Zero-Network Testing Pattern

All on-chain guardrails implement dependency injection via the `EvmRpcClient` trait.
In production, `HttpRpcClient` executes live JSON-RPC 2.0 calls.
In testing, `MockRpcClient` provides deterministic, millisecond responses without touching live networks or incurring rate limits.
