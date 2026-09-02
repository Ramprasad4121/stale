---
"@ramprasad4121/stale": minor
---

Introduced `checkPrices` for highly optimized multicall batching, drastically reducing RPC calls when checking multiple feeds. `checkPrice` now uses `checkPrices` internally.
