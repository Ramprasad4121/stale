---
"@ramprasad4121/stale": minor
---

Introduced three powerful new guardrails:

- `checkPriceDeviation`: Multi-oracle price comparison. Queries two independent Chainlink feeds for the same asset and blocks if they deviate beyond a configurable threshold — the gold-standard defense against flash-loan oracle manipulation.
- `checkDeadline`: Ensures swap deadlines are reasonable. Blocks if the deadline is expired (stale intent replay), too tight (will expire before confirmation), or too far in the future (long-lived MEV risk).
- `AddressBook`: Configurable contract allowlist. If an address is not explicitly approved, the agent cannot interact with it. The simplest and most powerful access control.
