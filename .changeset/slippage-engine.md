---
"@ramprasad4121/stale": minor
---

Introduced `calculateMinAmountOut`, a native bigint slippage engine. It utilizes Chainlink Data Feeds to dynamically compute safe, mathematically guaranteed exact `minAmountOut` boundaries for DEX swaps, completely mitigating MEV sandwich attacks on autonomous agents.
