---
"@ramprasad4121/stale": minor
---

Introduced `checkMevRpc` and `checkIsContract` guardrails.
`checkMevRpc` ensures an agent is strictly using a known private, MEV-protected RPC endpoint before trading, preventing front-running and sandwich attacks in the public mempool.
`checkIsContract` ensures an agent is strictly interacting with a deployed smart contract, preventing EOA phishing attacks where agents are tricked into approving or sending funds to scammer wallets.
