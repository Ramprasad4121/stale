use stale::prelude::*;
use stale::types::{Decision, GuardrailResult};

#[derive(Clone)]
pub struct StaleService {}

impl StaleService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn check_price(
        &self,
        rpc_url: &str,
        feed: &str,
        max_age_seconds: u64,
        amount_eth: Option<f64>,
    ) -> GuardrailResult {
        // Resolve feed: symbol like ETH/USD -> address via registry
        // BLOCK on unknown feed - do NOT silently substitute (fail-closed)
        let feed_addr = if feed.contains('/') {
            let symbol_upper = feed.to_uppercase();
            if let Some(entry) = stale::feeds::REGISTRY.iter().find(|e| e.symbol.to_uppercase() == symbol_upper) {
                entry.address.to_string()
            } else {
                return GuardrailResult::block(format!(
                    "Unknown feed symbol: {} - not allowlisted - BLOCK (run GET /v1/feeds for allowlisted set)",
                    feed
                ));
            }
        } else if feed.starts_with("0x") || feed.starts_with("0X") {
            // Validate address is in allowlist - lookup will fail in check_price if not
            feed.to_string()
        } else {
            // Non-address, non-symbol input -> BLOCK, do not fallback to DEFAULT_FEED
            return GuardrailResult::block(format!(
                "Invalid feed input: {} - must be symbol like ETH/USD or 0x address - BLOCK",
                feed
            ));
        };

        // Fail-closed: always call real check, never simulate ALLOW
        // RPC failures, decode failures, stale data all yield BLOCK per core library
        let rpc = HttpRpcClient::new(rpc_url);
        let input = CheckPriceInput {
            feed: &feed_addr,
            max_age_seconds: max_age_seconds as i64,
            amount_eth,
            now_seconds: None,
        };

        let result = check_price(&rpc, input).await;
        
        if result.decision == Decision::Allow {
            GuardrailResult::allow(result.reason)
                .with_metadata(serde_json::json!({
                    "price_usd": result.price_usd,
                    "age_seconds": result.age_seconds,
                    "feed": feed_addr,
                }))
        } else {
            GuardrailResult::block(result.reason)
                .with_metadata(serde_json::json!({
                    "feed": feed_addr,
                    "age_seconds": result.age_seconds,
                }))
        }
    }

    pub async fn check_gas_price(
        &self,
        rpc_url: &str,
        max_gas_gwei: u64,
    ) -> GuardrailResult {
        // Fail-closed: real check only, no simulation bypass
        let rpc = HttpRpcClient::new(rpc_url);
        check_gas_price(&rpc, max_gas_gwei).await
    }

    pub async fn check_gas_price_1559(
        &self,
        rpc_url: &str,
        max_base_fee_gwei: u64,
        max_priority_fee_gwei: u64,
    ) -> GuardrailResult {
        // Fail-closed: real EIP-1559 check
        let rpc = HttpRpcClient::new(rpc_url);
        check_gas_price_1559(&rpc, max_base_fee_gwei, max_priority_fee_gwei).await
    }

    pub async fn check_sequencer(
        &self,
        rpc_url: &str,
        chain_id: u64,
    ) -> GuardrailResult {
        // Fail-closed: real sequencer check via Chainlink uptime feed
        let rpc = HttpRpcClient::new(rpc_url);
        let now = chrono::Utc::now().timestamp() as u64;
        // check_sequencer returns Option<String>: None = Allow, Some(reason) = Block
        if let Some(reason) = stale::sequencer::check_sequencer(chain_id, &rpc, now).await {
            GuardrailResult::block(reason)
        } else {
            GuardrailResult::allow(format!("Sequencer up on chain {} - OK", chain_id))
        }
    }

    pub async fn check_mev_rpc(
        &self,
        rpc_url: &str,
    ) -> GuardrailResult {
        // check_mev_rpc is synchronous offline check (allowlist of private RPCs)
        check_mev_rpc(rpc_url)
    }

    pub async fn check_chain_id(
        &self,
        rpc_url: &str,
        expected_chain_id: u64,
    ) -> GuardrailResult {
        // Fail-closed: real chain_id check
        let rpc = HttpRpcClient::new(rpc_url);
        check_chain_id(&rpc, expected_chain_id).await
    }

    pub async fn check_is_contract(
        &self,
        rpc_url: &str,
        address: &str,
    ) -> GuardrailResult {
        // Fail-closed: real bytecode check
        let rpc = HttpRpcClient::new(rpc_url);
        check_is_contract(&rpc, address).await
    }

    pub async fn run_pipeline(
        &self,
        rpc_url: String,
        checks: Vec<(String, String, serde_json::Value)>,
        fail_fast: bool,
    ) -> (stale::pipeline::PipelineResult, Vec<(String, GuardrailResult, f64)>) {
        let mode = if fail_fast {
            PipelineMode::FailFast
        } else {
            PipelineMode::RunAll
        };

        let mut pipeline = create_guard_pipeline(mode, None);

        for (name, check_type, config) in checks {
            let rpc_url_clone = rpc_url.clone();
            let check_type_clone = check_type.clone();
            let config_clone = config.clone();

            pipeline.add(name, move || {
                let rpc_url = rpc_url_clone.clone();
                let check_type = check_type_clone.clone();
                let config = config_clone.clone();
                async move {
                    Self::execute_check_static(&rpc_url, &check_type, &config).await
                }
            });
        }

        let report = pipeline.run().await;
        
        let mut results_with_timing = Vec::new();
        for guard_report in &report.results {
            results_with_timing.push((
                guard_report.name.clone(),
                GuardrailResult {
                    decision: guard_report.decision,
                    reason: guard_report.reason.clone(),
                    allow_execute: guard_report.decision == Decision::Allow,
                    metadata: None,
                },
                guard_report.duration_ms,
            ));
        }

        (report, results_with_timing)
    }

    async fn execute_check_static(
        rpc_url: &str,
        check_type: &str,
        config: &serde_json::Value,
    ) -> GuardrailResult {
        let service = StaleService::new();
        match check_type {
            "price" | "oracle" => {
                let feed = config.get("feed").and_then(|v| v.as_str()).unwrap_or("");
                if feed.is_empty() {
                    return GuardrailResult::block("Missing feed in config - BLOCK (must provide symbol like ETH/USD or 0x address)");
                }
                let max_age = config.get("max_age_seconds").and_then(|v| v.as_u64()).unwrap_or(60);
                let amount = config.get("amount_eth").and_then(|v| v.as_f64());
                service.check_price(rpc_url, feed, max_age, amount).await
            }
            "gas" | "gas_price" => {
                let max_gas = config.get("max_gas_gwei").and_then(|v| v.as_u64()).unwrap_or(50);
                if max_gas == 0 {
                    return GuardrailResult::block("max_gas_gwei must be > 0 - BLOCK");
                }
                service.check_gas_price(rpc_url, max_gas).await
            }
            "gas_1559" => {
                let base = config.get("max_base_fee_gwei").and_then(|v| v.as_u64()).unwrap_or(100);
                let priority = config.get("max_priority_fee_gwei").and_then(|v| v.as_u64()).unwrap_or(5);
                if base == 0 || priority == 0 {
                    return GuardrailResult::block("max_base_fee_gwei and max_priority_fee_gwei must be > 0 - BLOCK");
                }
                service.check_gas_price_1559(rpc_url, base, priority).await
            }
            "sequencer" => {
                let chain_id = config.get("chain_id").and_then(|v| v.as_u64()).unwrap_or(42161);
                service.check_sequencer(rpc_url, chain_id).await
            }
            "mev" | "mev_rpc" => {
                service.check_mev_rpc(rpc_url).await
            }
            "chain_id" => {
                let expected = config.get("expected_chain_id").and_then(|v| v.as_u64()).unwrap_or(1);
                service.check_chain_id(rpc_url, expected).await
            }
            "is_contract" | "contract" => {
                let addr = config.get("address").and_then(|v| v.as_str()).unwrap_or("");
                if addr.is_empty() {
                    return GuardrailResult::block("Missing address in config for is_contract check - BLOCK");
                }
                service.check_is_contract(rpc_url, addr).await
            }
            _ => GuardrailResult::block(format!(
                "Unknown check type: {} - BLOCK (allowed: price, gas, gas_1559, sequencer, mev, chain_id, is_contract). See GET /v1/pipeline/status for supported guards",
                check_type
            )),
        }
    }
}

impl Default for StaleService {
    fn default() -> Self {
        Self::new()
    }
}
