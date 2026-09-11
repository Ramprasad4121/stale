use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub description: String,
    pub version: u32,
    pub environment: String, // production, staging, development
    pub is_active: bool,
    pub is_default: bool,
    pub rules: PolicyRules,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRules {
    pub fail_mode: FailMode,
    pub guard_timeout_ms: u64,
    pub max_guards: usize,
    pub oracle: OracleRules,
    pub gas: GasRules,
    pub network: NetworkRules,
    pub liquidity: LiquidityRules,
    pub permissions: PermissionRules,
    pub compliance: ComplianceRules,
    pub custom_guards: Vec<CustomGuard>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailMode {
    FailFast,   // block on first failure - fail-closed
    FailClosed, // run all, block if any fails - enterprise default, fail-closed
    // WarnOnly removed: product is ALLOW-or-BLOCK only per fail-closed contract
    // For audit-only evaluation, use ?audit_only=true query param which logs but does NOT allow execution
    // This preserves fail-closed: audit_only mode never returns ALLOW for execution, only for analysis
}

impl FailMode {
    pub fn is_fail_closed(&self) -> bool {
        matches!(self, FailMode::FailFast | FailMode::FailClosed)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleRules {
    pub enabled: bool,
    pub max_age_seconds: u64,
    pub feeds: Vec<String>, // feed addresses or symbols
    pub require_deviation_check: bool,
    pub max_deviation_bps: u32, // basis points
    pub fallback_feeds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasRules {
    pub enabled: bool,
    pub max_gas_gwei: u64,
    pub max_priority_fee_gwei: Option<u64>,
    pub max_base_fee_gwei: Option<u64>,
    pub block_on_high_gas: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRules {
    pub enabled: bool,
    pub allowed_chain_ids: Vec<u64>,
    pub require_mev_protection: bool,
    pub allowed_rpc_hosts: Vec<String>,
    pub check_sequencer: bool,
    pub sequencer_grace_seconds: u64,
    pub check_rpc_sync: bool,
    pub max_rpc_drift_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityRules {
    pub enabled: bool,
    pub check_pools: bool,
    pub min_liquidity_usd: Option<f64>,
    pub check_slippage: bool,
    pub max_slippage_bps: u32,
    pub check_deadline: bool,
    pub max_deadline_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRules {
    pub enforce_allowlist: bool,
    pub allowlist: HashMap<String, String>, // name -> address
    pub block_unbounded_approvals: bool,
    pub check_allowance: bool,
    pub check_balance: bool,
    pub check_is_contract: bool,
    pub rate_limit_per_min: u32,
    pub spending_cap_usd_per_hour: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceRules {
    pub check_sanctions: bool,
    pub sanctions_oracle: Option<String>,
    pub require_audit_log: bool,
    pub block_paused_contracts: bool,
    pub check_honeypot: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomGuard {
    pub name: String,
    pub enabled: bool,
    pub config: serde_json::Value,
}

impl Default for PolicyRules {
    fn default() -> Self {
        Self {
            fail_mode: FailMode::FailClosed,
            guard_timeout_ms: 5000,
            max_guards: 32,
            oracle: OracleRules {
                enabled: true,
                max_age_seconds: 60,
                feeds: vec!["ETH/USD".to_string()],
                require_deviation_check: true,
                max_deviation_bps: 100, // 1%
                fallback_feeds: vec![],
            },
            gas: GasRules {
                enabled: true,
                max_gas_gwei: 50,
                max_priority_fee_gwei: Some(5),
                max_base_fee_gwei: Some(100),
                block_on_high_gas: true,
            },
            network: NetworkRules {
                enabled: true,
                allowed_chain_ids: vec![1, 10, 42161, 8453], // ETH, OP, ARB, BASE
                require_mev_protection: true,
                allowed_rpc_hosts: vec![],
                check_sequencer: true,
                sequencer_grace_seconds: 3600,
                check_rpc_sync: true,
                max_rpc_drift_seconds: 60,
            },
            liquidity: LiquidityRules {
                enabled: true,
                check_pools: true,
                min_liquidity_usd: Some(10000.0),
                check_slippage: true,
                max_slippage_bps: 100, // 1%
                check_deadline: true,
                max_deadline_seconds: 1800,
            },
            permissions: PermissionRules {
                enforce_allowlist: true,
                allowlist: HashMap::new(),
                block_unbounded_approvals: true,
                check_allowance: true,
                check_balance: true,
                check_is_contract: true,
                rate_limit_per_min: 60,
                spending_cap_usd_per_hour: Some(100000.0),
            },
            compliance: ComplianceRules {
                check_sanctions: true,
                sanctions_oracle: None,
                require_audit_log: true,
                block_paused_contracts: true,
                check_honeypot: true,
            },
            custom_guards: vec![],
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePolicyRequest {
    pub name: String,
    pub description: Option<String>,
    pub environment: String,
    pub rules: Option<PolicyRules>,
    pub is_default: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub rules: Option<PolicyRules>,
    pub is_active: Option<bool>,
    pub is_default: Option<bool>,
}
