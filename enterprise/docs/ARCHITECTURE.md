# Stale Enterprise Architecture

## Decision Log

### Why Rust for API?
- Stale core is Rust → zero FFI overhead, share types (Decision, GuardrailResult)
- Axum + Tokio = <15ms p95 for pipeline of 4 guards
- Fail-closed guarantee preserved: no GC pauses hiding RPC failures

### Why Next.js for Dashboard?
- Enterprise buyers expect polished UI, not CLI
- Recharts for block rate, gas saved
- Real-time via polling (prod: SSE/WebSocket from audit service)

### Data Model Choices
- **DashMap** in-memory for demo → swap to Postgres + sqlx in prod (feature flag)
- **Audit logs**: In-memory Vec filtered → prod: ClickHouse (columnar, 365-day, cheap)
- **Rate limiting**: Use stale::RateLimiter per org (already battle-tested) + Redis for distributed

## Fail-Closed Invariants (Preserved)

1. `allow_execute == (decision == ALLOW)` — enforced in GuardrailResult constructors
2. Any RPC error → BLOCK, not ALLOW
3. Unknown feed symbol → BLOCK with hint to /v1/feeds
4. Missing policy → use default strict policy, not permissive
5. Negative max_age → usage error (exit 2 in CLI, 400 in API), not BLOCK

## Multi-Tenancy Isolation

- Org ID in every table, enforced in service layer (not just query param)
- API key → org_id lookup cached in Redis (5min TTL)
- Audit logs: org_id filter mandatory, else 0 results
- Rate limit: per org + per API key (1000/min Enterprise default)

## Policy Engine

```
Policy {
  rules: {
    fail_mode: FailClosed | FailFast // WarnOnly removed - contradicts fail-closed, use audit_only flag for eval
    oracle: { max_age, feeds[], deviation_check, max_deviation_bps }
    gas: { max_gas_gwei, max_priority, block_on_high }
    network: { allowed_chain_ids, require_mev, allowed_rpc_hosts, check_sequencer }
    liquidity: { check_pools, min_liquidity_usd, max_slippage_bps }
    permissions: { enforce_allowlist, allowlist{}, rate_limit, spending_cap }
    compliance: { check_sanctions, block_paused, check_honeypot }
  }
  version: u32 // increment on rules change
  environment: prod|staging|dev
  is_default: bool // one per env per org
}
```

Pipeline runner:
1. Resolve policy (by ID or default for env)
2. Build checks from policy.rules OR from request.checks (request overrides)
3. Validate max_guards <= 64 (Enterprise)
4. Run via stale::create_guard_pipeline(mode)
5. Collect per-guard results for audit
6. If BLOCK and org has webhook_url → dispatch async (tokio::spawn, 5s timeout)

## Security

- API keys: `stale_{env}_{uuid_hex}` → bcrypt hash stored, prefix visible for identification
- Rotation: create new, revoke old after 24h grace
- IP allowlist: checked in middleware (prod: Cloudflare + middleware)
- SSO: SAML via WorkOS/Auth0 (roadmap) — sso_enabled flag in org.settings
- PII: rpc_url hashed (sha256) in audit logs, never stored raw
- Audit logs: append-only, no UPDATE/DELETE API, only INSERT + SELECT

## Scaling

- Stateless API → horizontal via K8s HPA (CPU 70%)
- ClickHouse for logs → 100k logs/day * 365 = 36.5M rows, ~10GB compressed
- Redis for rate limits → 1k ops/sec per org, pipeline: INCR + EXPIRE
- Postgres for orgs/policies → <10k rows, not bottleneck

## Observability

- Tracing: tracing-subscriber JSON, trace_id = audit.trace_id
- Metrics: Prometheus at /metrics
  - stale_checks_total{decision, env}
  - stale_check_duration_ms histogram
  - stale_gas_saved_usd counter
- Logs: structured JSON, shipped to Datadog
- Alerts: Slack webhook on BLOCK rate >10% in 5min

## Billing

- Stripe metered billing: report usage daily via Stripe Usage Records
- Checks counted per org per day, aggregated monthly
- Overage: $0.005/check after included
- Gas saved: marketing metric, not billed, but shows ROI

## On-Prem

- Docker image: `ghcr.io/stale-enterprise/api:1.0.0`
- Helm chart: deploys API + Postgres + Redis + ClickHouse
- Air-gapped: no external calls except RPCs (customer-provided)
- License key: JWT with org_id, tier, expiry, checked offline
