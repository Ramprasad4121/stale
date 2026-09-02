---
"@ramprasad4121/stale": minor
---

Introduced `checkBalance` (Solvency Guardrail) and `checkPaused` (Protocol State Guardrail). Agents can now natively verify they hold sufficient funds before attempting trades, preventing wasted gas and network spam from failing transactions. Additionally, agents can natively verify that target protocols (e.g., USDC, Aave) are not paused by their multi-sigs before attempting interaction.
