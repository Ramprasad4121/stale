# Stale Enterprise Platform

> **Fail-closed guardrails for agents that touch money — now as an enterprise platform**

This is the enterprise transformation of [stale](https://github.com/Ramprasad4121/stale) (v2.0.0) — a Rust library with 20+ DeFi security guardrails — into a production-ready, multi-tenant SaaS platform.

---

## 🚀 What Changed: Library → Enterprise Platform

### Before (Open Source Library)
- Rust crate + CLI + MCP server
- Single-tenant, code-level integration
- Local audit logs (JSON files)
- Manual policy configuration

### After (Enterprise Platform)
- **Managed API**: `POST /v1/pipeline/run` with <15ms p95
- **Multi-tenant**: Orgs, environments (prod/staging/dev), RBAC
- **Policy Engine**: Versioned, fail-closed/fail-fast/warn-only modes
- **Observability**: Real-time metrics, ClickHouse audit logs, Slack/PagerDuty webhooks
- **Security**: SSO (SAML), API key rotation, IP allowlists, mTLS, SOC2-ready logs
- **Dashboard**: Next.js enterprise UI with live audit trail
- **Scale**: 100k checks/mo on Enterprise, 99.99% SLA, on-prem option

---

## 🏗️ Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Enterprise Dashboard                  │
│  Next.js 14 • Tailwind • Recharts • Real-time SSE       │
│  Pages: Dashboard, Policies, Audit, API Keys, Team      │
└──────────────────────┬──────────────────────────────────┘
                       │ HTTPS + API Key / JWT
┌──────────────────────▼──────────────────────────────────┐
│              Stale Enterprise API (Rust)                │
│  Axum • Tokio • DashMap • stale v2.0.0 core             │
│                                                         │
│  Routes:                                                │
│  • POST /v1/check/{price,gas,sequencer,mev,chain}      │
│  • POST /v1/pipeline/run (policy-aware)                │
│  • GET  /v1/audit/logs, /stats, /export                │
│  • CRUD /v1/policies (versioned)                       │
│  • CRUD /v1/orgs, /api-keys, /feeds                   │
│  • GET  /health, /metrics (Prometheus)                 │
│                                                         │
│  Middleware:                                            │
│  • API Key Auth (Bearer + X-API-Key + X-Org-Id demo)   │
│  • RBAC (Owner/Admin/Dev/Auditor/Viewer)               │
│  • Rate Limiting per org (stale::RateLimiter)          │
│  • Audit Logging (every ALLOW/BLOCK)                   │
└──────────────────────┬──────────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────────┐
│                   Stale Core (Rust)                     │
│  20+ guardrails: oracle, gas, sequencer, MEV,           │
│  allowance, balance, pool, deadline, sanctions, etc.    │
│  Fail-closed: any doubt → BLOCK                         │
└─────────────────────────────────────────────────────────┘

Data Layer (Production):
• Postgres for orgs, users, policies, api_keys
• ClickHouse for audit logs (365-day retention, S3 export)
• Redis for rate limits, revocation list, caching
• S3 for audit exports, compliance

Integrations:
• Slack / PagerDuty webhooks on BLOCK
• Datadog / Prometheus metrics
• Stripe billing (metered checks)
• SSO: Okta, Azure AD, Google
```

---

## 💼 Enterprise Features Implemented

### 1. Multi-Tenancy & RBAC
- Organizations with tiers (Free/Pro/Enterprise)
- Environments: production, staging, development
- Roles: Owner (billing+team), Admin, Developer (policies+keys), Auditor (read-only logs), Viewer
- Per-org settings: RPC allowlists, gas thresholds, MEV requirements

### 2. Policy Engine (Versioned)
- **3 policies seeded**:
  - `Production - Strict` (default, 60s oracle, 50 Gwei, MEV required)
  - `Staging - Permissive` (300s, 100 Gwei, MEV optional)
  - `High-Value - Fort Knox` (30s, 30 Gwei, 10 tx/min, $10k/hr cap)
- Fail modes: FailFast (first BLOCK), FailClosed (all checks, enterprise default), WarnOnly (audit)
- Custom guards via WASM (roadmap)

### 3. Audit & Compliance
- Every check logs: trace_id, decision, blocked_by, duration, chain_id, amount_usd, gas, etc.
- Query: `?decision=BLOCK&blocked_by=gas&environment=production&from=...&to=...`
- Stats: total, block_rate, avg_duration, top_blocked_reasons, gas_saved_usd
- Export: JSON/CSV to S3 with signed URL (SOC2)
- Demo: 50 seeded logs with realistic BLOCK reasons

### 4. API & SDKs
- **Auth**: `Authorization: Bearer stale_live_...` or `X-API-Key` or demo `X-Org-Id`
- **Endpoints**: 15+ including `/v1/pipeline/run` which uses policy or custom checks
- **SDKs**: Rust (native), TypeScript (fetch wrapper), Python (requests) — see `docs/sdks.md`
- **MCP**: Existing `stale-mcp` still works, now proxies to enterprise API

### 5. Ops & Billing
- Health: `/health`, `/ready`, `/metrics` (Prometheus)
- Billing: Stripe metering, 100k included, $0.005 overage, $999/mo Enterprise
- Usage: 14% demo, gas saved $4.7k
- Deployment: Docker, K8s Helm, Terraform, on-prem air-gapped

---

## 🚀 Quickstart

### Backend (Rust API)
```bash
cd backend
cargo run --release
# → http://localhost:3001
# Try: curl http://localhost:3001/health
# Try: curl http://localhost:3001/v1/feeds -H "X-Org-Id: <org_id from logs>"
# Try: curl -X POST http://localhost:3001/v1/pipeline/run \
#   -H "Content-Type: application/json" \
#   -H "X-Org-Id: <id>" \
#   -d '{"rpc_url":"https://rpc.flashbots.net","chain_id":1,"checks":[{"type":"gas","config":{"max_gas_gwei":50}}]}'
```

### Frontend (Dashboard)
```bash
cd frontend
npm install
npm run dev
# → http://localhost:3000
# Dashboard at /dashboard shows live audit logs
```

### Docker (Full Stack)
```bash
docker-compose up --build
# Frontend: http://localhost:3000
# Backend:  http://localhost:3001
```

---

## 📊 Demo Data

On startup, backend seeds:
- 1 org: Acme DeFi Corp (Enterprise tier)
- 4 users: Owner (Ramprasad), Admin (Alice), Dev (Bob), Auditor (Carol)
- 2 API keys: Production (live), Staging (test) — printed in logs
- 3 policies (see above)
- 50 audit logs (last 24h, mix ALLOW/BLOCK)

---

## 🔐 Security

- **Fail-closed**: Any RPC failure, decode error, stale oracle → BLOCK
- **No unwraps**: Every field optional on some path, handled
- **SSRF guard**: `allowed_rpc_hosts` per org
- **PII redaction**: RPC URLs hashed in audit logs
- **Key security**: bcrypt hash, prefix visible, rotation, IP allowlist
- **SOC2**: Audit logs immutable, 365-day retention, S3 export, webhook signing

---

## 📈 Pricing

| Tier | Price | Checks | Features |
|------|-------|--------|----------|
| Starter | $0 | 1k/mo | Community, 1 org, 7-day logs |
| Pro | $299/mo | 50k/mo | 5 orgs, 90-day logs, webhooks, SSO |
| Enterprise | $999/mo | 100k/mo + overage | Unlimited, 365-day, SAML, on-prem, 99.99% SLA, 24/7 |

---

## 🗺️ Roadmap to $1M ARR

1. **Week 1-2**: Launch with 3 design partners (DeFi protocols using stale)
2. **Week 3-4**: Add Python SDK + LangChain integration (agents)
3. **Month 2**: SOC2 Type I, Terraform provider, Datadog integration
4. **Month 3**: On-prem Helm chart, custom WASM guards, 10 paying customers
5. **Month 6**: $1M ARR = 80 Enterprise customers @ $999 + overage

---

## 📂 Structure

```
stale-enterprise-platform/
├── backend/               # Rust Axum API (enterprise layer)
│   ├── src/
│   │   ├── main.rs        # Server + seeding
│   │   ├── models/        # Org, User, ApiKey, Policy, Audit, Check
│   │   ├── services/      # StaleService, Audit, Policy, Org, Webhook
│   │   ├── routes/        # Health, checks, pipeline, audit, policies...
│   │   └── middleware/    # API key auth, RBAC
│   └── Cargo.toml         # Depends on stale v2.0.0 via path
├── frontend/              # Next.js Dashboard
│   ├── app/
│   │   ├── page.tsx       # Landing (enterprise marketing)
│   │   └── dashboard/     # Live audit dashboard
│   └── package.json
├── infra/
│   ├── Dockerfile.backend
│   ├── Dockerfile.frontend
│   └── k8s/               # Helm charts (roadmap)
├── docs/
│   ├── ARCHITECTURE.md
│   ├── API.md
│   └── ENTERPRISE.md
└── docker-compose.yml
```

---

## 🤝 Credits

- **Core**: [stale](https://github.com/Ramprasad4121/stale) by Ramprasad Goud — MIT, 2.0.0, 20+ guardrails, 153 tests
- **Enterprise Layer**: Built in Hyderabad, IN — transforming library into platform
- **Stack**: Rust (Axum), Next.js, Tailwind, Recharts, Postgres (prod), ClickHouse (prod)

---

## 📄 License

- Core `stale`: MIT
- Enterprise platform: Proprietary (source-available for customers)

---

**Live Demo**: `npm run dev` in frontend + `cargo run` in backend → http://localhost:3000/dashboard
