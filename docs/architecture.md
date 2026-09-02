# `stale` Architecture

## Overview

The `stale` project serves as a crucial guardrail for on-chain AI agents. Its primary goal is to verify that Chainlink Data Feed prices are fresh and valid before an agent takes any action. It provides a programmatic library, a CLI tool, and a Model Context Protocol (MCP) server, allowing Large Language Model (LLM) agents to safely verify oracle data without requiring transaction execution capabilities.

## Architecture Philosophy: Strict Fail-Closed

The central tenet of the `stale` architecture is a **strict fail-closed** model. If any step in the price fetching or validation process is incomplete, invalid, or unavailable, the system defaults to returning a `BLOCK` decision.

Key fail-closed mechanisms include:

- **Timestamp Validation:** If `updatedAt` is missing, unparseable, `0n`, or in the future (indicating clock skew or spoofing), it fails closed.
- **Data Integrity:** Negative or zero prices (`answer <= 0n`) and incomplete rounds (`answeredInRound < roundId`) immediately trigger a `BLOCK`.
- **Feed Allowlist:** Queries to unknown or unsupported proxy addresses fail closed to prevent agents from being tricked into querying malicious contracts.
- **Chain Validation:** RPC responses are cross-checked against the expected `chainId` for the requested data feed.

This approach guarantees that an `ALLOW` decision is only issued when all preconditions—correct chain, valid data, and fresh timestamps—have been fully met.

## Interaction with Chainlink Data Feeds via `viem`

`stale` interfaces with Chainlink Data Feeds using `viem` as its Ethereum interaction library.

- **Read-Only:** The interaction strictly uses `client.readContract`. There is no wallet integration, private key management, or transaction signing capability in the system, eliminating the risk of accidental state mutations.
- **Data Fetched:** The guardrail leverages the official Chainlink Data Feed ABI to fetch two key pieces of data:
  - `decimals()`
  - `latestRoundData()` (which returns `roundId`, `answer`, `startedAt`, `updatedAt`, and `answeredInRound`)
- **Parallel Execution:** It executes parallel reads using `Promise.all` for efficiency, taking advantage of `viem`'s built-in `eth_call` batching capabilities at the end of the tick.

## LLM Agent Integration via MCP Server

The project includes an MCP server implementation that safely exposes the `stale` guardrail to LLM agents.

- **Standardized Tools:** The MCP server registers three specific tools that agents can invoke:
  - `stale_isStale`: Performs pure freshness logic given timestamps without network calls. Safe for unit-tests and offline verification.
  - `stale_quote`: Executes pure price math based on the data feed `answer` and `decimals`.
  - `stale_check`: Performs the full verification pipeline (viem network lookup → freshness check → price quote math).
- **Safe Exposure:** The MCP implementation strictly limits feeds to an allowlist (e.g., ETH/USD and BTC/USD on mainnet) and propagates the fail-closed behavior safely to the LLM.
- **JSON Serialization & Data Transport:** Because the Model Context Protocol uses JSON—which cannot natively serialize JavaScript `bigint` values—the MCP adapter intelligently normalizes types, converting `bigint` data like `answer` and `updatedAt` to `string` formats, ensuring robust communication between the TypeScript logic and the LLM agent.

By keeping the business logic purely analytical and sandboxed behind read-only viem queries, the MCP server provides LLMs with a mathematically sound and secure guardrail to verify Chainlink data freshness.
