use crate::models::audit::{AuditLog, AuditQuery, AuditStats, BlockedReasonStat, TimeBucket, AuditRequest, AuditMetadata, AuditGuardResult};
use chrono::{Utc, Duration};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use stale::types::Decision;

#[derive(Clone)]
pub struct AuditService {
    logs: Arc<DashMap<Uuid, AuditLog>>,
}

impl AuditService {
    pub fn new() -> Self {
        Self {
            logs: Arc::new(DashMap::new()),
        }
    }

    pub async fn log(&self, log: AuditLog) {
        self.logs.insert(log.id, log);
    }

    pub async fn query(&self, query: AuditQuery) -> Vec<AuditLog> {
        let mut results: Vec<AuditLog> = self.logs
            .iter()
            .filter(|entry| {
                let log = entry.value();
                if let Some(org_id) = query.org_id {
                    if log.org_id != org_id {
                        return false;
                    }
                }
                if let Some(ref decision) = query.decision {
                    let decision_upper = decision.to_uppercase();
                    let log_decision = format!("{:?}", log.decision).to_uppercase();
                    if log_decision != decision_upper {
                        return false;
                    }
                }
                if let Some(ref blocked_by) = query.blocked_by {
                    if let Some(ref log_blocked) = log.blocked_by {
                        if !log_blocked.to_lowercase().contains(&blocked_by.to_lowercase()) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                if let Some(ref env) = query.environment {
                    if &log.environment != env {
                        return false;
                    }
                }
                if let Some(from) = query.from {
                    if log.created_at < from {
                        return false;
                    }
                }
                if let Some(to) = query.to {
                    if log.created_at > to {
                        return false;
                    }
                }
                if let Some(ref search) = query.search {
                    let search_lower = search.to_lowercase();
                    if !log.reason.to_lowercase().contains(&search_lower)
                        && !log.trace_id.to_lowercase().contains(&search_lower)
                    {
                        return false;
                    }
                }
                true
            })
            .map(|e| e.value().clone())
            .collect();

        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        let offset = query.offset.unwrap_or(0) as usize;
        let limit = query.limit.unwrap_or(100) as usize;

        results.into_iter().skip(offset).take(limit).collect()
    }

    pub async fn get_stats(&self, org_id: Uuid) -> AuditStats {
        let logs: Vec<AuditLog> = self.logs
            .iter()
            .filter(|e| e.value().org_id == org_id)
            .map(|e| e.value().clone())
            .collect();

        let total_checks = logs.len() as u64;
        let total_blocks = logs.iter().filter(|l| l.decision == Decision::Block).count() as u64;
        let total_allows = total_checks - total_blocks;
        let block_rate = if total_checks > 0 {
            total_blocks as f64 / total_checks as f64
        } else {
            0.0
        };

        let avg_duration_ms = if total_checks > 0 {
            logs.iter().map(|l| l.duration_ms).sum::<f64>() / total_checks as f64
        } else {
            0.0
        };

        let mut reason_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
        for log in logs.iter().filter(|l| l.decision == Decision::Block) {
            let key = log.blocked_by.clone().unwrap_or_else(|| "unknown".to_string());
            *reason_counts.entry(key).or_insert(0) += 1;
        }

        let mut top_blocked: Vec<BlockedReasonStat> = reason_counts
            .into_iter()
            .map(|(guard, count)| BlockedReasonStat {
                guard,
                count,
                percentage: if total_blocks > 0 {
                    count as f64 / total_blocks as f64 * 100.0
                } else {
                    0.0
                },
            })
            .collect();
        top_blocked.sort_by(|a, b| b.count.cmp(&a.count));
        top_blocked.truncate(5);

        let mut hourly: std::collections::HashMap<String, (u64, u64)> = std::collections::HashMap::new();
        let now = Utc::now();
        for i in 0..24 {
            let hour = now - Duration::hours(i);
            let key = hour.format("%Y-%m-%d %H:00").to_string();
            hourly.insert(key, (0, 0));
        }

        for log in &logs {
            let hour_key = log.created_at.format("%Y-%m-%d %H:00").to_string();
            if let Some((allows, blocks)) = hourly.get_mut(&hour_key) {
                if log.decision == Decision::Allow {
                    *allows += 1;
                } else {
                    *blocks += 1;
                }
            }
        }

        let mut checks_per_hour: Vec<TimeBucket> = hourly
            .into_iter()
            .map(|(k, (allows, blocks))| {
                let timestamp = chrono::DateTime::parse_from_str(&format!("{} +00:00", k.replace(" ", "T")), "%Y-%m-%dT%H:00 %z")
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                TimeBucket {
                    timestamp,
                    allows,
                    blocks,
                }
            })
            .collect();
        checks_per_hour.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        // NOTE: gas_saved_usd is DEMO-ONLY estimate, never SOC2/compliance
        // In production, this would be calculated from actual prevented losses
        // and real gas costs from on-chain data, stored in ClickHouse
        // For demo: each BLOCK prevents ~$47.50 avg gas + potential loss
        // This is NOT a compliance metric - marked as estimate
        let gas_saved_usd = total_blocks as f64 * 47.5;

        AuditStats {
            total_checks,
            total_blocks,
            total_allows,
            block_rate,
            avg_duration_ms,
            top_blocked_reasons: top_blocked,
            checks_per_hour,
            gas_saved_usd,
        }
    }

    pub async fn get_by_id(&self, id: Uuid) -> Option<AuditLog> {
        self.logs.get(&id).map(|e| e.value().clone())
    }

    // Seed with demo data - DEMO-ONLY, never SOC2/compliance
    // Production uses ClickHouse with real on-chain observed values
    // Seeded values like 2500.0, 19000000 are synthetic for UI demo only
    pub async fn seed_demo(&self, org_id: Uuid) {
        use chrono::Duration as ChronoDuration;
        let now = Utc::now();
        
        let decisions = vec![
            (Decision::Allow, None, "All guardrails passed - execution allowed"),
            (Decision::Block, Some("gas_price"), "Gas price 78 Gwei exceeds policy 50 Gwei - BLOCK"),
            (Decision::Allow, None, "Price feed ETH/USD fresh - 12s age"),
            (Decision::Block, Some("oracle_freshness"), "Chainlink ETH/USD stale - 187s > 60s max - BLOCK"),
            (Decision::Block, Some("mev_rpc"), "Public RPC detected - not MEV protected - BLOCK"),
            (Decision::Allow, None, "Sequencer up, pool liquidity OK"),
            (Decision::Block, Some("sequencer"), "Arbitrum sequencer down - grace period active - BLOCK"),
            (Decision::Allow, None, "Allowance and balance checks passed"),
        ];

        for i in 0..50 {
            let (decision, blocked_by, reason) = &decisions[i % decisions.len()];
            let id = Uuid::new_v4();
            let created_at = now - ChronoDuration::minutes((i * 23) as i64);
            
            // DEMO-ONLY synthetic data - never used for SOC2/compliance
            // Production would use real observed values from check_price, eth_blockNumber, etc
            // Values like 2500.0, 19000000, 25.0 are synthetic for UI demonstration
            let log = AuditLog {
                id,
                org_id,
                user_id: None,
                api_key_id: Some(Uuid::new_v4()),
                trace_id: format!("trc_{}", hex::encode(&id.as_bytes()[0..8])),
                environment: if i % 3 == 0 { "production".to_string() } else if i % 3 == 1 { "staging".to_string() } else { "development".to_string() },
                policy_id: Some(Uuid::new_v4()),
                policy_version: Some(3),
                decision: decision.clone(),
                blocked_by: blocked_by.map(|s| s.to_string()),
                reason: reason.to_string(),
                duration_ms: 15.0 + (i as f64 * 1.3) % 120.0,
                request: AuditRequest {
                    method: "POST".to_string(),
                    path: "/v1/pipeline/run".to_string(),
                    chain_id: Some(1),
                    target_address: Some("0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string()),
                    rpc_url_hash: Some("sha256:demo_hash".to_string()), // DEMO-ONLY - prod uses real SHA256 of RPC URL
                    amount_usd: Some(10000.0 + (i as f64 * 100.0)),
                    ip_address: None, // PII redacted for demo
                    user_agent: Some("stale-sdk-rust/2.0.0".to_string()),
                },
                results: vec![
                    AuditGuardResult {
                        guard_name: "oracle_freshness".to_string(),
                        decision: if blocked_by == &Some("oracle_freshness") { Decision::Block } else { Decision::Allow },
                        reason: "Oracle check".to_string(),
                        duration_ms: 12.5,
                        metadata: None,
                    },
                    AuditGuardResult {
                        guard_name: "gas_price".to_string(),
                        decision: if blocked_by == &Some("gas_price") { Decision::Block } else { Decision::Allow },
                        reason: "Gas check".to_string(),
                        duration_ms: 8.2,
                        metadata: None,
                    },
                ],
                metadata: AuditMetadata {
                    // DEMO-ONLY synthetic observed values - production uses real on-chain data
                    gas_price_gwei: Some(25.0 + (i as f64 % 60.0)),
                    block_number: Some(19000000 + i as u64 * 100), // DEMO-ONLY
                    sequencer_up: Some(blocked_by != &Some("sequencer")),
                    mev_protected: Some(blocked_by != &Some("mev_rpc")),
                    price_usd: Some(2500.0 + (i as f64 % 100.0)), // DEMO-ONLY synthetic
                    price_age_seconds: Some(if blocked_by == &Some("oracle_freshness") { 187 } else { 12 }),
                    webhook_dispatched: *decision == Decision::Block,
                    slack_dispatched: *decision == Decision::Block,
                },
                created_at,
            };
            self.logs.insert(id, log);
        }
    }
}

impl Default for AuditService {
    fn default() -> Self {
        Self::new()
    }
}
