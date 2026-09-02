---
"@ramprasad4121/stale": minor
---

Introduced `RateLimiter` and `SpendingCap` guardrails. These are pure in-memory, zero-dependency classes that enforce transaction frequency limits and cumulative spending caps over rolling time windows, preventing runaway AI agents from draining wallets or DOSing protocols.
