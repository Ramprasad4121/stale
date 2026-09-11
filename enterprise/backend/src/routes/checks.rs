use axum::{extract::State, Json, Extension, http::StatusCode};
use serde_json::{json, Value};
use crate::AppState;
use crate::middleware::auth::{AuthContext, enforce_scope};
use crate::models::check::{PriceCheckRequest, GasCheckRequest, GenericCheckRequest};
use crate::models::api_key::ApiKeyScope;
use sha2::{Sha256, Digest};
use std::time::Instant;

pub async fn check_price(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<PriceCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::CheckRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: check_read"})))
    })?;

    let start = Instant::now();
    let feed = req.feed.unwrap_or_else(|| "ETH/USD".to_string());
    let max_age = req.max_age_seconds.unwrap_or(60);
    
    let result = state.stale_service.check_price(&req.rpc_url, &feed, max_age, req.amount_eth).await;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    
    let response = json!({
        "trace_id": format!("trc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        "decision": format!("{:?}", result.decision).to_uppercase(),
        "allow_execute": result.decision == stale::types::Decision::Allow,
        "reason": result.reason,
        "metadata": result.metadata,
        "feed": feed,
        "max_age_seconds": max_age,
        "org_id": auth.org_id,
        "duration_ms": duration_ms,
        "timestamp": chrono::Utc::now(),
    });

    let mut hasher = Sha256::new();
    hasher.update(req.rpc_url.as_bytes());
    let hash_result = hasher.finalize();
    let rpc_url_hash = format!("sha256:{}", hex::encode(&hash_result[..8]));

    let observed_price_usd = result.metadata.as_ref().and_then(|m| m.get("price_usd")).and_then(|v| v.as_f64());
    let amount_usd = match (req.amount_eth, observed_price_usd) {
        (Some(amount_eth), Some(price_usd)) => Some(amount_eth * price_usd),
        _ => None,
    };

    let audit_log = crate::models::audit::AuditLog {
        id: uuid::Uuid::new_v4(),
        org_id: auth.org_id,
        user_id: auth.user_id,
        api_key_id: auth.api_key_id,
        trace_id: response["trace_id"].as_str().unwrap_or("unknown").to_string(),
        environment: auth.environment.clone(),
        policy_id: None,
        policy_version: None,
        decision: result.decision,
        blocked_by: if result.decision == stale::types::Decision::Block { Some("price".to_string()) } else { None },
        reason: result.reason.clone(),
        duration_ms,
        request: crate::models::audit::AuditRequest {
            method: "POST".to_string(),
            path: "/v1/check/price".to_string(),
            chain_id: Some(1),
            target_address: None,
            rpc_url_hash: Some(rpc_url_hash),
            amount_usd,
            ip_address: None,
            user_agent: None,
        },
        results: vec![],
        metadata: crate::models::audit::AuditMetadata {
            gas_price_gwei: None,
            block_number: None,
            sequencer_up: None,
            mev_protected: None,
            price_usd: observed_price_usd,
            price_age_seconds: result.metadata.as_ref().and_then(|m| m.get("age_seconds")).and_then(|v| v.as_u64()),
            webhook_dispatched: result.decision == stale::types::Decision::Block,
            slack_dispatched: false,
        },
        created_at: chrono::Utc::now(),
    };
    state.audit_service.log(audit_log).await;

    Ok(Json(response))
}

pub async fn check_gas(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<GasCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::CheckRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: check_read"})))
    })?;

    let start = Instant::now();
    let max_gas = req.max_gas_gwei.unwrap_or(50);
    let result = state.stale_service.check_gas_price(&req.rpc_url, max_gas).await;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    
    Ok(Json(json!({
        "trace_id": format!("trc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        "decision": format!("{:?}", result.decision).to_uppercase(),
        "allow_execute": result.decision == stale::types::Decision::Allow,
        "reason": result.reason,
        "metadata": result.metadata,
        "max_gas_gwei": max_gas,
        "duration_ms": duration_ms,
        "org_id": auth.org_id,
        "timestamp": chrono::Utc::now(),
    })))
}

pub async fn check_sequencer(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<GenericCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::CheckRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: check_read"})))
    })?;

    let start = Instant::now();
    let chain_id = req.chain_id.unwrap_or(42161);
    let result = state.stale_service.check_sequencer(&req.rpc_url, chain_id).await;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    
    Ok(Json(json!({
        "trace_id": format!("trc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        "decision": format!("{:?}", result.decision).to_uppercase(),
        "allow_execute": result.decision == stale::types::Decision::Allow,
        "reason": result.reason,
        "chain_id": chain_id,
        "duration_ms": duration_ms,
        "org_id": auth.org_id,
        "timestamp": chrono::Utc::now(),
    })))
}

pub async fn check_mev(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<GenericCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::CheckRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: check_read"})))
    })?;

    let start = Instant::now();
    let result = state.stale_service.check_mev_rpc(&req.rpc_url).await;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    
    Ok(Json(json!({
        "trace_id": format!("trc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        "decision": format!("{:?}", result.decision).to_uppercase(),
        "allow_execute": result.decision == stale::types::Decision::Allow,
        "reason": result.reason,
        "is_mev_protected": result.decision == stale::types::Decision::Allow,
        "duration_ms": duration_ms,
        "org_id": auth.org_id,
        "timestamp": chrono::Utc::now(),
    })))
}

pub async fn check_chain(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<GenericCheckRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::CheckRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: check_read"})))
    })?;

    let start = Instant::now();
    let expected = req.chain_id.unwrap_or(1);
    let result = state.stale_service.check_chain_id(&req.rpc_url, expected).await;
    let duration_ms = start.elapsed().as_secs_f64() * 1000.0;
    
    Ok(Json(json!({
        "trace_id": format!("trc_{}", uuid::Uuid::new_v4().to_string()[..8].to_string()),
        "decision": format!("{:?}", result.decision).to_uppercase(),
        "allow_execute": result.decision == stale::types::Decision::Allow,
        "reason": result.reason,
        "expected_chain_id": expected,
        "duration_ms": duration_ms,
        "org_id": auth.org_id,
        "timestamp": chrono::Utc::now(),
    })))
}
