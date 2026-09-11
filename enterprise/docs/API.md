# Stale Enterprise API Reference

Base URL: `https://api.stale.sh` (prod) or `http://localhost:3001` (local)

Auth: `Authorization: Bearer stale_live_...` or `X-API-Key: stale_live_...` (bcrypt-verified, RBAC scopes enforced, no X-Org-Id bypass - fail-closed)

## Health

```
GET /health
GET /ready
GET /metrics (Prometheus)
```

## Checks (Single Guard)

### Price
```
POST /v1/check/price
{
  "rpc_url": "https://rpc.flashbots.net",
  "feed": "ETH/USD", // or 0x address
  "max_age_seconds": 60,
  "amount_eth": 1.0
}
→ { decision: "ALLOW", reason: "...", metadata: { price_usd, age_seconds } }
```

### Gas
```
POST /v1/check/gas
{ "rpc_url": "...", "max_gas_gwei": 50 }
```

### Sequencer
```
POST /v1/check/sequencer
{ "rpc_url": "https://arb1.arbitrum.io/rpc" }
```

### MEV
```
POST /v1/check/mev
{ "rpc_url": "https://rpc.flashbots.net" }
```

### Chain ID
```
POST /v1/check/chain
{ "rpc_url": "...", "chain_id": 1 }
```

## Pipeline (Multi-Guard, Policy-Aware)

```
POST /v1/pipeline/run
{
  "rpc_url": "https://rpc.flashbots.net",
  "policy_id": "prod-strict-v3" or UUID, // optional, uses default if omitted
  "environment": "production", // prod|staging|dev
  "chain_id": 1,
  "checks": [ // optional if policy provides
    { "type": "price", "config": { "feed": "ETH/USD", "max_age_seconds": 60 } },
    { "type": "gas", "config": { "max_gas_gwei": 50 } },
    { "type": "mev", "config": {} },
    { "type": "sequencer", "config": {} }
  ],
  "context": {
    "target_address": "0xE592...",
    "amount_usd": 50000
  }
}

→ {
  "trace_id": "trc_...",
  "decision": "BLOCK",
  "blocked_by": "gas_price",
  "reason": "Gas 78 > 50",
  "duration_ms": 43.2,
  "results": [
    { "guard_name": "oracle_eth_usd", "decision": "ALLOW", "duration_ms": 12 },
    { "guard_name": "gas_price", "decision": "BLOCK", "duration_ms": 8 }
  ],
  "policy_id": "...",
  "policy_version": 3,
  "audit_log_id": "..."
}
```

Fail modes (ALLOW-or-BLOCK only, fail-closed):
- FailClosed (default, enterprise): run all checks, BLOCK if any fails
- FailFast: stop on first BLOCK, return immediately
- Note: WarnOnly removed - contradicts fail-closed. For audit-only evaluation, use ?audit_only=true (logs but never ALLOWs execution in prod)

## Audit

```
GET /v1/audit/logs?decision=BLOCK&blocked_by=gas&environment=production&limit=100&search=sequencer
GET /v1/audit/logs/:id
GET /v1/audit/stats → { total_checks, total_blocks, block_rate, avg_duration_ms, top_blocked_reasons[], gas_saved_usd }
GET /v1/audit/export?from=...&to=... → { export_id, download_url, count }
```

## Policies

```
GET /v1/policies?environment=production
POST /v1/policies { name, description, environment, rules?, is_default? }
GET /v1/policies/:id
PATCH /v1/policies/:id { name?, rules?, is_active?, is_default? }
DELETE /v1/policies/:id
GET /v1/policies/:id/versions
```

Rules schema: see `models/policy.rs` — oracle, gas, network, liquidity, permissions, compliance

## Orgs

```
GET /v1/orgs
POST /v1/orgs { name, slug?, tier? }
GET /v1/orgs/:id → { org, members_count, api_keys_count, stats }
PATCH /v1/orgs/:id { name?, settings? }
GET /v1/orgs/members
POST /v1/orgs/members/invite { email, role }
GET /v1/orgs/billing → { plan, usage, next_invoice }
```

## API Keys

```
GET /v1/api-keys → list (no raw keys)
POST /v1/api-keys { name, scopes[], environment, expires_in_days?, rate_limit_per_min? }
→ { id, api_key: "stale_live_...", key_prefix, message: "Save once!" }

POST /v1/api-keys/:id/revoke
POST /v1/api-keys/:id/rotate → { old_key_id, new_key: { id, api_key } }
```

Scopes: check_read, check_write, pipeline_run, audit_read, policies_read, policies_write, admin

## Feeds

```
GET /v1/feeds → { feeds: [{ symbol, base, address, chain, decimals }], total, default_feed }
GET /v1/feeds/:symbol → { symbol, base, address, chain, found }
```

## Errors

- 401: Missing/invalid API key
- 403: Org mismatch or insufficient scope
- 400: No checks, too many guards (>64), invalid feed
- 429: Rate limited (per org + per key)
- 500: Internal, but fail-closed → logs BLOCK

## Webhooks

On BLOCK, if org.settings.webhook_url set:

```
POST {webhook_url}
Headers: X-Stale-Event: guardrail.blocked, X-Stale-Version: 2.0.0
Body: { event: "guardrail.blocked", trace_id, org_id, decision, blocked_by, reason, environment, timestamp, request: { chain_id, target } }
```

Slack: formatted blocks with decision, blocked_by, trace_id, reason

## SDKs

### Rust
```rust
use stale_enterprise::Client;
let client = Client::new("stale_live_...", "https://api.stale.sh");
let result = client.pipeline().run(PipelineRequest { rpc_url: "...", .. }).await?;
if result.decision == "BLOCK" { return Err(...) }
```

### TypeScript
```ts
import { Stale } from "@stale-enterprise/sdk";
const stale = new Stale({ apiKey: process.env.STALE_API_KEY });
const { decision, blocked_by } = await stale.pipeline.run({
  rpc_url: "https://rpc.flashbots.net",
  policy_id: "prod-strict",
  context: { amount_usd: 50000 }
});
if (decision === "BLOCK") throw new Error(`Blocked by ${blocked_by}`);
```

### Python
```py
from stale_enterprise import Stale
client = Stale(api_key="stale_live_...")
result = client.pipeline.run(rpc_url="...", chain_id=1)
assert result.decision == "ALLOW"
```
