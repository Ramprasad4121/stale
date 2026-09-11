use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct PipelineRunRequest {
    pub policy_id: Option<String>, // UUID or name
    pub environment: Option<String>,
    pub rpc_url: String,
    pub chain_id: Option<u64>,
    pub checks: Vec<CheckRequest>,
    pub context: Option<CheckContext>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CheckRequest {
    #[serde(rename = "type")]
    pub check_type: String, // price, gas, sequencer, mev, allowance, etc
    pub config: serde_json::Value,
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CheckContext {
    pub target_address: Option<String>,
    pub token_address: Option<String>,
    pub amount: Option<String>,
    pub amount_usd: Option<f64>,
    pub from_address: Option<String>,
    pub spender: Option<String>,
    pub deadline: Option<u64>,
    pub slippage_bps: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct PipelineRunResponse {
    pub trace_id: String,
    pub decision: String,
    pub blocked_by: Option<String>,
    pub reason: String,
    pub duration_ms: f64,
    pub results: Vec<CheckResult>,
    pub policy_id: Option<String>,
    pub policy_version: Option<u32>,
    pub audit_log_id: String,
}

#[derive(Debug, Serialize)]
pub struct CheckResult {
    pub guard_name: String,
    pub check_type: String,
    pub decision: String,
    pub reason: String,
    pub duration_ms: f64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct PriceCheckRequest {
    pub rpc_url: String,
    pub feed: Option<String>, // ETH/USD or address
    pub max_age_seconds: Option<u64>,
    pub amount_eth: Option<f64>,
}

#[derive(Debug, Deserialize)]
pub struct GasCheckRequest {
    pub rpc_url: String,
    pub max_gas_gwei: Option<u64>,
    pub max_priority_fee_gwei: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct GenericCheckRequest {
    pub rpc_url: String,
    pub chain_id: Option<u64>,
    pub config: Option<serde_json::Value>,
}
