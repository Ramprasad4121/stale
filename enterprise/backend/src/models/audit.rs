use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use stale::types::{Decision, GuardrailResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub org_id: Uuid,
    pub user_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub trace_id: String,
    pub environment: String,
    pub policy_id: Option<Uuid>,
    pub policy_version: Option<u32>,
    pub decision: Decision,
    pub blocked_by: Option<String>,
    pub reason: String,
    pub duration_ms: f64,
    pub request: AuditRequest,
    pub results: Vec<AuditGuardResult>,
    pub metadata: AuditMetadata,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRequest {
    pub method: String,
    pub path: String,
    pub chain_id: Option<u64>,
    pub target_address: Option<String>,
    pub rpc_url_hash: Option<String>, // hashed for privacy
    pub amount_usd: Option<f64>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditGuardResult {
    pub guard_name: String,
    pub decision: Decision,
    pub reason: String,
    pub duration_ms: f64,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetadata {
    pub gas_price_gwei: Option<f64>,
    pub block_number: Option<u64>,
    pub sequencer_up: Option<bool>,
    pub mev_protected: Option<bool>,
    pub price_usd: Option<f64>,
    pub price_age_seconds: Option<u64>,
    pub webhook_dispatched: bool,
    pub slack_dispatched: bool,
}

#[derive(Debug, Deserialize)]
pub struct AuditQuery {
    pub org_id: Option<Uuid>,
    pub decision: Option<String>, // ALLOW, BLOCK
    pub blocked_by: Option<String>,
    pub environment: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub search: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AuditStats {
    pub total_checks: u64,
    pub total_blocks: u64,
    pub total_allows: u64,
    pub block_rate: f64,
    pub avg_duration_ms: f64,
    pub top_blocked_reasons: Vec<BlockedReasonStat>,
    pub checks_per_hour: Vec<TimeBucket>,
    pub gas_saved_usd: f64,
}

#[derive(Debug, Serialize)]
pub struct BlockedReasonStat {
    pub guard: String,
    pub count: u64,
    pub percentage: f64,
}

#[derive(Debug, Serialize)]
pub struct TimeBucket {
    pub timestamp: DateTime<Utc>,
    pub allows: u64,
    pub blocks: u64,
}

impl From<GuardrailResult> for AuditGuardResult {
    fn from(r: GuardrailResult) -> Self {
        Self {
            guard_name: "unknown".to_string(),
            decision: r.decision,
            reason: r.reason,
            duration_ms: 0.0,
            metadata: r.metadata,
        }
    }
}
