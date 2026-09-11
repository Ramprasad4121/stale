use axum::{extract::State, Json, Extension, http::StatusCode};
use serde_json::{json, Value};
use crate::AppState;
use crate::middleware::auth::{AuthContext, enforce_scope};
use crate::models::check::{PipelineRunRequest, PipelineRunResponse, CheckResult};
use crate::models::audit::{AuditLog, AuditRequest, AuditMetadata, AuditGuardResult};
use crate::models::api_key::ApiKeyScope;
use uuid::Uuid;
use chrono::Utc;
use stale::types::Decision;
use sha2::{Sha256, Digest};

pub async fn run_pipeline(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<PipelineRunRequest>,
) -> Result<Json<PipelineRunResponse>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PipelineRun).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: pipeline_run"})))
    })?;

    let trace_id = format!("trc_{}", &Uuid::new_v4().to_string()[..12]);
    let start = std::time::Instant::now();

    let policy = if let Some(policy_id_str) = &req.policy_id {
        if let Ok(policy_id) = Uuid::parse_str(policy_id_str) {
            state.policy_service.get(policy_id).await
        } else {
            let policies = state.policy_service.list(auth.org_id, req.environment.clone()).await;
            policies.into_iter().find(|p| p.name == *policy_id_str)
        }
    } else {
        let env = req.environment.clone().unwrap_or_else(|| "production".to_string());
        state.policy_service.get_default(auth.org_id, &env).await
    };

    let checks_to_run: Vec<(String, String, Value)> = if !req.checks.is_empty() {
        req.checks.into_iter()
            .filter(|c| c.enabled.unwrap_or(true))
            .enumerate()
            .map(|(i, c)| {
                let name = format!("check_{}_{}", c.check_type, i);
                (name, c.check_type, c.config)
            })
            .collect()
    } else if let Some(ref pol) = policy {
        let mut checks = Vec::new();
        if pol.rules.oracle.enabled {
            for feed in &pol.rules.oracle.feeds {
                checks.push((
                    format!("oracle_{}", feed.replace('/', "_")),
                    "price".to_string(),
                    json!({
                        "feed": feed,
                        "max_age_seconds": pol.rules.oracle.max_age_seconds,
                    })
                ));
            }
        }
        if pol.rules.gas.enabled {
            checks.push((
                "gas_price".to_string(),
                "gas".to_string(),
                json!({
                    "max_gas_gwei": pol.rules.gas.max_gas_gwei,
                })
            ));
        }
        if pol.rules.network.enabled {
            if pol.rules.network.require_mev_protection {
                checks.push(("mev_protection".to_string(), "mev".to_string(), json!({})));
            }
            if pol.rules.network.check_sequencer {
                checks.push(("sequencer".to_string(), "sequencer".to_string(), json!({
                    "chain_id": req.chain_id.unwrap_or(1)
                })));
            }
            for chain_id in &pol.rules.network.allowed_chain_ids {
                if *chain_id == req.chain_id.unwrap_or(1) {
                    checks.push((
                        format!("chain_{}", chain_id),
                        "chain_id".to_string(),
                        json!({"expected_chain_id": chain_id})
                    ));
                    break;
                }
            }
        }
        checks
    } else {
        vec![
            ("oracle_eth_usd".to_string(), "price".to_string(), json!({"feed": "ETH/USD", "max_age_seconds": 60})),
            ("gas_price".to_string(), "gas".to_string(), json!({"max_gas_gwei": 50})),
            ("mev_protection".to_string(), "mev".to_string(), json!({})),
            ("sequencer".to_string(), "sequencer".to_string(), json!({"chain_id": req.chain_id.unwrap_or(1)})),
        ]
    };

    if checks_to_run.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "No checks to run",
                "message": "Provide checks array or configure a policy"
            }))
        ));
    }

    if checks_to_run.len() > 64 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Too many checks",
                "message": "Enterprise tier allows max 64 guards per pipeline"
            }))
        ));
    }

    let fail_fast = policy.as_ref().map(|p| matches!(p.rules.fail_mode, crate::models::policy::FailMode::FailFast)).unwrap_or(false);
    
    let (pipeline_result, detailed_results) = state.stale_service.run_pipeline(
        req.rpc_url.clone(),
        checks_to_run,
        fail_fast,
    ).await;

    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;

    let check_results: Vec<CheckResult> = detailed_results.iter().map(|(name, result, dur)| {
        CheckResult {
            guard_name: name.clone(),
            check_type: name.split('_').next().unwrap_or("unknown").to_string(),
            decision: format!("{:?}", result.decision).to_uppercase(),
            reason: result.reason.clone(),
            duration_ms: *dur,
            metadata: result.metadata.clone(),
        }
    }).collect();

    let audit_results: Vec<AuditGuardResult> = detailed_results.iter().map(|(name, result, dur)| {
        AuditGuardResult {
            guard_name: name.clone(),
            decision: result.decision.clone(),
            reason: result.reason.clone(),
            duration_ms: *dur,
            metadata: result.metadata.clone(),
        }
    }).collect();

    let response = PipelineRunResponse {
        trace_id: trace_id.clone(),
        decision: format!("{:?}", pipeline_result.decision).to_uppercase(),
        blocked_by: pipeline_result.blocked_by.clone(),
        reason: pipeline_result.reason.clone(),
        duration_ms,
        results: check_results,
        policy_id: policy.as_ref().map(|p| p.id.to_string()),
        policy_version: policy.as_ref().map(|p| p.version),
        audit_log_id: Uuid::new_v4().to_string(),
    };

    let mut hasher = Sha256::new();
    hasher.update(req.rpc_url.as_bytes());
    let hash_result = hasher.finalize();
    let rpc_url_hash = format!("sha256:{}", hex::encode(&hash_result[..8]));

    let mut observed_gas_price: Option<f64> = None;
    let mut observed_price_usd: Option<f64> = None;
    let mut observed_price_age: Option<u64> = None;
    let mut observed_sequencer_up: Option<bool> = None;
    let mut observed_mev_protected: Option<bool> = None;

    for (_, result, _) in &detailed_results {
        if let Some(meta) = &result.metadata {
            if let Some(gas) = meta.get("gas_price_gwei").and_then(|v| v.as_f64()) {
                observed_gas_price = Some(gas);
            }
            if let Some(price) = meta.get("price_usd").and_then(|v| v.as_f64()) {
                observed_price_usd = Some(price);
            }
            if let Some(age) = meta.get("age_seconds").and_then(|v| v.as_u64()) {
                observed_price_age = Some(age);
            }
        }
        if result.reason.to_lowercase().contains("sequencer") {
            observed_sequencer_up = Some(result.decision == Decision::Allow);
        }
        if result.reason.to_lowercase().contains("mev") {
            observed_mev_protected = Some(result.decision == Decision::Allow);
        }
    }

    let audit_log = AuditLog {
        id: Uuid::new_v4(),
        org_id: auth.org_id,
        user_id: auth.user_id,
        api_key_id: auth.api_key_id,
        trace_id: trace_id.clone(),
        environment: req.environment.clone().unwrap_or_else(|| "production".to_string()),
        policy_id: policy.as_ref().map(|p| p.id),
        policy_version: policy.as_ref().map(|p| p.version),
        decision: pipeline_result.decision.clone(),
        blocked_by: pipeline_result.blocked_by.clone(),
        reason: pipeline_result.reason.clone(),
        duration_ms,
        request: AuditRequest {
            method: "POST".to_string(),
            path: "/v1/pipeline/run".to_string(),
            chain_id: req.chain_id,
            target_address: req.context.as_ref().and_then(|c| c.target_address.clone()),
            rpc_url_hash: Some(rpc_url_hash),
            amount_usd: req.context.as_ref().and_then(|c| c.amount_usd),
            ip_address: None,
            user_agent: Some("stale-enterprise-sdk/1.0.0".to_string()),
        },
        results: audit_results,
        metadata: AuditMetadata {
            gas_price_gwei: observed_gas_price,
            block_number: None,
            sequencer_up: observed_sequencer_up,
            mev_protected: observed_mev_protected,
            price_usd: observed_price_usd,
            price_age_seconds: observed_price_age,
            webhook_dispatched: pipeline_result.decision == Decision::Block,
            slack_dispatched: pipeline_result.decision == Decision::Block,
        },
        created_at: Utc::now(),
    };

    state.audit_service.log(audit_log.clone()).await;

    if pipeline_result.decision == Decision::Block {
        if let Some(org) = state.org_service.get_org(auth.org_id).await {
            let webhook_payload = json!({
                "event": "guardrail.blocked",
                "trace_id": trace_id,
                "org_id": auth.org_id,
                "decision": "BLOCK",
                "blocked_by": pipeline_result.blocked_by,
                "reason": pipeline_result.reason,
                "environment": req.environment.unwrap_or_else(|| "production".to_string()),
                "timestamp": Utc::now(),
                "request": {
                    "chain_id": req.chain_id,
                    "target": req.context.as_ref().and_then(|c| c.target_address.clone()),
                }
            });
            state.webhook_service.dispatch_block_event(
                org.settings.webhook_url,
                org.settings.slack_webhook_url,
                webhook_payload,
            ).await;
        }
    }

    Ok(Json(response))
}

pub async fn get_pipeline_status(
    State(_state): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
) -> Json<Value> {
    Json(json!({
        "status": "operational",
        "supported_guards": [
            "price",
            "gas",
            "gas_1559",
            "sequencer",
            "mev",
            "chain_id",
            "is_contract"
        ],
        "planned_guards": [
            "allowance",
            "balance",
            "approval",
            "pool_v2",
            "pool_v3",
            "deadline",
            "slippage",
            "sanctioned",
            "paused",
            "token_tax",
            "nonce",
            "rpc_sync"
        ],
        "max_guards_per_pipeline": 64,
        "default_timeout_ms": 5000,
        "enterprise_features": {
            "fail_modes": ["fail_fast", "fail_closed"],
            "audit_only_mode": "Available via ?audit_only=true for non-production evaluation (does not allow execution, only logs)",
            "custom_guards": false,
            "webhooks": true,
            "audit_logs": true
        }
    }))
}
