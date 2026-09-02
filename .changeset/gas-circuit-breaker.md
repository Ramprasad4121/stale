---
"@ramprasad4121/stale": minor
---

Introduced `checkGasPrice`, a network gas spike circuit breaker. It actively polls the base fee over RPC and fails closed (blocks execution) if network congestion causes gas prices to spike beyond the agent's safe threshold. This prevents AI agents from unknowingly burning disproportionate amounts of their treasury on transaction fees during gas wars.
