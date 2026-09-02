---
"@ramprasad4121/stale": minor
---

Introduced `checkApproval`, a strict allowance guardrail. Prevents AI agents from executing `approve` transactions with `MaxUint256` or dangerously large bounds, enforcing "Exact Amount Approvals" to prevent complete treasury drains on compromised routers.
