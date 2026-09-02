---
"@ramprasad4121/stale": minor
---

Introduced a robust Network State Guardrail suite.

- `checkRpcSync`: Fails closed if the agent's RPC node falls out of sync, preventing the agent from acting on deeply stale or fake blockchain states.
- `checkChainId`: Strictly enforces that the RPC matches the agent's expected execution environment, preventing catastrophic cross-chain replay attacks.
- `checkNonce`: Actively queries the network for the agent's nonce, blocking execution if the agent is desynced and trying to double-spend or re-use a stale nonce.
