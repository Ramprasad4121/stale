---
"@ramprasad4121/stale": minor
---

Introduced `checkTokenTax` — a honeypot token detection guardrail. Uses `eth_call` to simulate a token transfer from a known holder. If the transfer reverts (meaning the token cannot be sold/transferred), the token is flagged as a honeypot and execution is blocked. This is the #1 defense against the most common DeFi scam vector.
