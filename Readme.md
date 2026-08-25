# Awesome Chainlink AI Agents

A practical guide and curated resource list for building AI agents that can safely read, verify, and act on Chainlink infrastructure, and for understanding how Chainlink itself uses AI internally (CRE, NAVLink, ACE, Confidential Compute).

This repo exists because most "AI agents + Web3" content is either too shallow to build with, or too scattered across docs, blog posts, and Discord threads to actually learn from in order. This is meant to be the guide I wish existed when I started asking: how does an AI agent actually verify a number before acting on it on-chain?

## Who this is for

- Developers building AI agents that need to read verified on-chain data, trigger on-chain actions, or move value cross-chain
- Chainlink developers who want to understand where and how AI fits into Chainlink's own product stack
- Security researchers and auditors reviewing AI-agent-driven smart contract integrations
- Anyone who wants a structured path through this material instead of piecing it together from scattered sources

## How this repo is organized

```
docs/
  01-foundations/              Why AI agents need an oracle layer at all
  02-how-chainlink-uses-ai/    NAVLink, ACE, Confidential Compute, CRE
  03-building-agents-on-chainlink/   Practical patterns for agent-driven on-chain actions
  04-security-for-agent-builders/    Audit checklists and failure modes specific to agents
examples/                      Runnable code for each pattern in 03-
resources/
  AWESOME.md                   Curated list of tools, papers, docs, and other builders
```

Start with `docs/01-foundations/` if you're new to this space. Jump straight to `examples/` if you already understand the concepts and want working code.

## Start here

1. [Why AI Agents Need an Oracle Layer](docs/01-foundations/why-agents-need-oracles.md)
2. [How Chainlink Uses AI Internally](docs/02-how-chainlink-uses-ai/)
3. [Building Agents on Chainlink](docs/03-building-agents-on-chainlink/)
4. [Security for Agent Builders](docs/04-security-for-agent-builders/)
5. [Curated Resources](resources/AWESOME.md)

## Contributing

This is meant to grow with input from other builders working in this space. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to submit a doc, example, or resource addition.

Corrections are especially welcome. Chainlink's AI-related products (CRE, ACE, NAVLink, Confidential Compute) are evolving quickly, and anything here that goes stale should get flagged or fixed.

## About

Maintained by [Ramprasad Edigi](https://ramprasadgoud.dev), a smart contract security researcher and Chainlink Community Advocate. Built as a companion to a daily technical writing series on Chainlink's architecture.

Follow along on [X](https://x.com/0xramprasad)

