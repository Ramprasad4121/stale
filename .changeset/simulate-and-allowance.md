---
"@ramprasad4121/stale": minor
---

Introduced `simulateTx` and `checkAllowance` guardrails.
`simulateTx` natively executes `eth_call` to trace an agent's intended transaction against the current blockchain state, failing closed if it reverts (preventing honeypots and wasted gas).
`checkAllowance` ensures an agent actually has the required ERC20 allowance granted to a spender before attempting a trade, preventing reverts from insufficient approvals.
