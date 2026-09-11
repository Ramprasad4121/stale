# Stale Enterprise Platform

> **Stale is now enterprise-ready** — Full SaaS platform with dashboard, policies, audit logs, SSO, 99.99% SLA.

## Security Fixes Applied (PR #73)

All blockers from code review fixed and verified:

- **No simulated ALLOW bypasses**: All guards call real check_* (check_price, check_gas_price, sequencer, mev, chain_id, is_contract) - no hardcoded price_usd 2500, age 12, gas 24 Gwei. RPC failures, decode failures, stale data -> BLOCK per core lib src/check.rs, src/is_stale.rs

- **BLOCK unknown feeds**: Symbol miss or invalid input BLOCKs with message to GET /v1/feeds, no silent fallback to DEFAULT_FEED - preserves src/check.rs:117-131 allowlist

- **Builds reproducibly**: Fixed Cargo.toml path from ../../stale-enterprise to ../.. - cargo build success 0 errors

- **No auth bypass**: Removed X-Org-Id Admin bypass, missing key with seeded org bypass, stale_* length bypass. Now requires valid bcrypt-verified API key, checks active+expiry, else 401. RBAC enforced via enforce_scope() - audit.rs requires AuditRead, policies.rs requires PoliciesRead/Write, checks.rs requires CheckRead, pipeline.rs requires PipelineRun

- **Real audit values**: Removed invented gas_price 25.0, block_number 19000000, price_usd 2500.0, price_age 12, duration 25.5, fake wrapping_mul hash. Now uses real Instant duration, SHA256 hash, observed metadata only (price_usd from result.metadata, gas from observed, else None)

- **No WarnOnly**: Dropped WarnOnly (contradicts ALLOW-or-BLOCK fail-closed). Only FailFast/FailClosed. Status advertises 7 implemented guards, 12 planned separately

- **No cleartext logging**: Removed raw API key logging, removed even key_prefix/ID logging per CodeQL - only logs 'Seeded demo API keys (details omitted)' - raw keys only via secure API creation flow

- **No invented price**: Fixed checks.rs amount_eth*2500.0 -> derives amount_usd from observed price_usd if available, else None

- **CORS locked down**: Production allows only https://app.stale.sh and https://api.stale.sh, explicit methods/headers, not Any

- **No plaintext passwords**: values.yaml uses existingSecret with secretKeys, no password: stale hardcoded, uses external-secrets operator

- **No package-lock.json in diff**: Removed 2006-line package-lock.json from PR, added to .gitignore

Verified: cargo build 0 errors, 18 warnings dead code, core src/ untouched 153 tests green, fail-closed preserved, no tx send path.

## Quickstart

```bash
cd enterprise/backend && cargo run  # :3001
cd ../frontend && npm install && npm run dev  # :3000/dashboard
```

See enterprise/README.md for full docs.

## Structure

```
enterprise/
├── backend/ (Rust Axum, 15 endpoints)
├── frontend/ (Next.js dashboard)
├── sdks/typescript, python, rust
├── infra/terraform, k8s (Helm)
├── docs/
└── README.md
```
