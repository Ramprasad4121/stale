---
"@ramprasad4121/stale": minor
---

Introduced native on-chain DEX liquidity guardrails `checkPoolV2` and `checkPoolV3`. These strictly protect AI agents from executing trades in low-liquidity honeypot pools by evaluating active depth natively over RPC before a transaction is signed.
