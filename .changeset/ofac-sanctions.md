---
"@ramprasad4121/stale": minor
---

Introduced `checkSanctioned` OFAC Compliance Guardrail. `stale` now utilizes the official Chainlink Sanctions Oracle to ensure that autonomous AI agents never interact with sanctioned entities (e.g. Lazarus Group, Tornado Cash routers), protecting node operators and developers from severe compliance violations.
