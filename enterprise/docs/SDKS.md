# Stale Enterprise SDKs

## TypeScript

```ts
// npm install @stale-enterprise/sdk
import { Stale } from "@stale-enterprise/sdk";

const stale = new Stale({
  apiKey: process.env.STALE_API_KEY!, // stale_live_...
  baseUrl: "https://api.stale.sh",
  environment: "production",
});

// Simple check
const gas = await stale.check.gas({
  rpc_url: "https://rpc.flashbots.net",
  max_gas_gwei: 50,
});
if (gas.decision === "BLOCK") throw new Error(gas.reason);

// Pipeline with policy
const result = await stale.pipeline.run({
  rpc_url: "https://rpc.flashbots.net",
  policy_id: "prod-strict-v3",
  chain_id: 1,
  context: {
    target_address: "0xE592427A0AEce92De3Edee1F18E0157C05861564",
    amount_usd: 50000,
  },
});

if (result.decision === "BLOCK") {
  console.error(`Blocked by ${result.blocked_by}: ${result.reason}`);
  // Webhook already dispatched to Slack
  return;
}

// Continue to sign and broadcast
await wallet.sendTransaction(tx);
```

## Python

```py
# pip install stale-enterprise
from stale_enterprise import Stale

client = Stale(api_key="stale_live_...", environment="production")

# Check price feed
price_check = client.check.price(
    rpc_url="https://ethereum-rpc.publicnode.com",
    feed="ETH/USD",
    max_age_seconds=60
)
assert price_check.decision == "ALLOW"
print(f"ETH price: ${price_check.metadata['price_usd']}")

# Pipeline
result = client.pipeline.run(
    rpc_url="https://rpc.flashbots.net",
    chain_id=1,
    checks=[
        {"type": "price", "config": {"feed": "ETH/USD", "max_age_seconds": 60}},
        {"type": "gas", "config": {"max_gas_gwei": 50}},
        {"type": "mev", "config": {}},
    ],
    context={"amount_usd": 10000}
)

if result.decision == "BLOCK":
    # Audit logged, Slack alerted
    raise Exception(f"Transaction blocked: {result.reason}")
```

## Rust

```rust
use stale_enterprise::{Client, PipelineRequest, CheckRequest};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new(
        "stale_live_...",
        "https://api.stale.sh"
    );

    // Or use local core directly for low latency
    // use stale::prelude::*;
    // let rpc = HttpRpcClient::new("https://rpc.flashbots.net");
    // let result = check_price(&rpc, CheckPriceInput { ... }).await;

    // Enterprise pipeline (with audit, webhooks, policy)
    let req = PipelineRequest {
        rpc_url: "https://rpc.flashbots.net".to_string(),
        policy_id: Some("prod-strict-v3".to_string()),
        chain_id: Some(1),
        checks: vec![],
        context: Some(json!({"amount_usd": 50000})),
        environment: Some("production".to_string()),
    };

    let result = client.pipeline.run(req).await?;
    
    match result.decision.as_str() {
        "ALLOW" => println!("✅ All checks passed in {}ms", result.duration_ms),
        "BLOCK" => {
            eprintln!("🚨 BLOCKED by {}: {}", 
                result.blocked_by.unwrap_or_default(), 
                result.reason
            );
            std::process::exit(1);
        }
        _ => unreachable!(),
    }

    Ok(())
}
```

## LangChain Integration

```ts
import { StaleTool } from "@stale-enterprise/langchain";

const staleTool = new StaleTool({
  apiKey: process.env.STALE_API_KEY,
  policyId: "prod-strict",
});

// Add to agent
const agent = new Agent({
  tools: [staleTool, uniswapTool, ...],
  systemPrompt: `
  Before any transaction that moves money, you MUST call stale_guardrail.
  If it returns BLOCK, you MUST NOT proceed and must explain why.
  `,
});
```

## ElizaOS / Claude MCP

```json
{
  "mcpServers": {
    "stale-enterprise": {
      "command": "npx",
      "args": ["-y", "@stale-enterprise/mcp"],
      "env": {
        "STALE_API_KEY": "stale_live_...",
        "STALE_ENV": "production"
      }
    }
  }
}
```

Then in Claude:
```
User: Swap 1 ETH for USDC on Uniswap

Claude: I'll check guardrails first...
[Calls stale_check]
→ ALLOW: oracle fresh, gas 24 Gwei, MEV protected
→ Proceeding with swap
```

## OpenAPI

Import `openapi.json` (generated from Axum) into Postman.

Base: `https://api.stale.sh`

Auth: Bearer token

Endpoints: See API.md
