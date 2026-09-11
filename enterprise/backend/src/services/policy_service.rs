use crate::models::policy::{Policy, PolicyRules, CreatePolicyRequest, UpdatePolicyRequest};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct PolicyService {
    policies: Arc<DashMap<Uuid, Policy>>,
}

impl PolicyService {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(DashMap::new()),
        }
    }

    pub async fn create(&self, org_id: Uuid, user_id: Uuid, req: CreatePolicyRequest) -> Policy {
        let id = Uuid::new_v4();
        let now = Utc::now();
        
        let policy = Policy {
            id,
            org_id,
            name: req.name,
            description: req.description.unwrap_or_else(|| "Enterprise guardrail policy".to_string()),
            version: 1,
            environment: req.environment,
            is_active: true,
            is_default: req.is_default.unwrap_or(false),
            rules: req.rules.unwrap_or_default(),
            created_by: user_id,
            created_at: now,
            updated_at: now,
        };

        // If this is default, unset other defaults for same env
        if policy.is_default {
            let env = policy.environment.clone();
            let org = policy.org_id;
            for mut entry in self.policies.iter_mut() {
                if entry.org_id == org && entry.environment == env && entry.is_default {
                    entry.is_default = false;
                }
            }
        }

        self.policies.insert(id, policy.clone());
        policy
    }

    pub async fn list(&self, org_id: Uuid, environment: Option<String>) -> Vec<Policy> {
        let mut result: Vec<Policy> = self.policies
            .iter()
            .filter(|e| {
                e.org_id == org_id && 
                if let Some(ref env) = environment {
                    &e.environment == env
                } else {
                    true
                }
            })
            .map(|e| e.value().clone())
            .collect();
        result.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        result
    }

    pub async fn get(&self, id: Uuid) -> Option<Policy> {
        self.policies.get(&id).map(|e| e.value().clone())
    }

    pub async fn get_default(&self, org_id: Uuid, environment: &str) -> Option<Policy> {
        self.policies
            .iter()
            .find(|e| e.org_id == org_id && e.environment == environment && e.is_default && e.is_active)
            .map(|e| e.value().clone())
    }

    pub async fn update(&self, id: Uuid, req: UpdatePolicyRequest) -> Option<Policy> {
        if let Some(mut entry) = self.policies.get_mut(&id) {
            if let Some(name) = req.name {
                entry.name = name;
            }
            if let Some(desc) = req.description {
                entry.description = desc;
            }
            if let Some(rules) = req.rules {
                entry.rules = rules;
                entry.version += 1;
            }
            if let Some(active) = req.is_active {
                entry.is_active = active;
            }
            if let Some(is_default) = req.is_default {
                entry.is_default = is_default;
            }
            entry.updated_at = Utc::now();
            return Some(entry.clone());
        }
        None
    }

    pub async fn delete(&self, id: Uuid) -> bool {
        self.policies.remove(&id).is_some()
    }

    pub async fn seed_demo(&self, org_id: Uuid, user_id: Uuid) {
        let policies = vec![
            ("Production - Strict", "production", true, PolicyRules {
                oracle: crate::models::policy::OracleRules {
                    enabled: true,
                    max_age_seconds: 60,
                    feeds: vec!["ETH/USD".to_string(), "BTC/USD".to_string()],
                    require_deviation_check: true,
                    max_deviation_bps: 100,
                    fallback_feeds: vec!["ETH/USD:Arbitrum".to_string()],
                },
                gas: crate::models::policy::GasRules {
                    enabled: true,
                    max_gas_gwei: 50,
                    max_priority_fee_gwei: Some(3),
                    max_base_fee_gwei: Some(80),
                    block_on_high_gas: true,
                },
                ..Default::default()
            }),
            ("Staging - Permissive", "staging", false, PolicyRules {
                oracle: crate::models::policy::OracleRules {
                    enabled: true,
                    max_age_seconds: 300,
                    feeds: vec!["ETH/USD".to_string()],
                    require_deviation_check: false,
                    max_deviation_bps: 500,
                    fallback_feeds: vec![],
                },
                gas: crate::models::policy::GasRules {
                    enabled: true,
                    max_gas_gwei: 100,
                    max_priority_fee_gwei: Some(10),
                    max_base_fee_gwei: Some(200),
                    block_on_high_gas: false,
                },
                network: crate::models::policy::NetworkRules {
                    enabled: true,
                    allowed_chain_ids: vec![1, 10, 42161, 8453, 11155111],
                    require_mev_protection: false,
                    allowed_rpc_hosts: vec![],
                    check_sequencer: true,
                    sequencer_grace_seconds: 3600,
                    check_rpc_sync: false,
                    max_rpc_drift_seconds: 120,
                },
                ..Default::default()
            }),
            ("High-Value - Fort Knox", "production", false, PolicyRules {
                fail_mode: crate::models::policy::FailMode::FailClosed,
                guard_timeout_ms: 10000,
                max_guards: 64,
                oracle: crate::models::policy::OracleRules {
                    enabled: true,
                    max_age_seconds: 30,
                    feeds: vec!["ETH/USD".to_string(), "BTC/USD".to_string(), "USDC/USD".to_string()],
                    require_deviation_check: true,
                    max_deviation_bps: 50,
                    fallback_feeds: vec!["ETH/USD:Chainlink".to_string(), "ETH/USD:Chronicle".to_string()],
                },
                gas: crate::models::policy::GasRules {
                    enabled: true,
                    max_gas_gwei: 30,
                    max_priority_fee_gwei: Some(2),
                    max_base_fee_gwei: Some(50),
                    block_on_high_gas: true,
                },
                permissions: crate::models::policy::PermissionRules {
                    enforce_allowlist: true,
                    allowlist: std::collections::HashMap::from([
                        ("UNISWAP_V3_ROUTER".to_string(), "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                        ("WETH".to_string(), "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2".to_string()),
                    ]),
                    block_unbounded_approvals: true,
                    check_allowance: true,
                    check_balance: true,
                    check_is_contract: true,
                    rate_limit_per_min: 10,
                    spending_cap_usd_per_hour: Some(10000.0),
                },
                compliance: crate::models::policy::ComplianceRules {
                    check_sanctions: true,
                    sanctions_oracle: None,
                    require_audit_log: true,
                    block_paused_contracts: true,
                    check_honeypot: true,
                },
                ..Default::default()
            }),
        ];

        for (name, env, is_default, rules) in policies {
            let id = Uuid::new_v4();
            let now = Utc::now();
            let policy = Policy {
                id,
                org_id,
                name: name.to_string(),
                description: format!("Enterprise policy for {} environment", env),
                version: 1,
                environment: env.to_string(),
                is_active: true,
                is_default,
                rules,
                created_by: user_id,
                created_at: now,
                updated_at: now,
            };
            self.policies.insert(id, policy);
        }
    }
}

impl Default for PolicyService {
    fn default() -> Self {
        Self::new()
    }
}
