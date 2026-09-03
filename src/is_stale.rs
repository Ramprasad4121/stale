//! Pure offline freshness math: `age = now - updatedAt` vs policy.
//!
//! No network. Fail closed on missing/negative `updatedAt`, negative
//! `max_age_seconds`, arithmetic overflow, future timestamps
//! (`age < 0` → `not-yet-valid`, covering local-clock-behind-chain skew),
//! and `age > max_age_seconds` (stale).

use crate::types::Decision;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Freshness verdict with observed `age_seconds` (`None` when unparseable).
pub struct IsStaleResult {
    pub decision: Decision,
    pub age_seconds: Option<i64>,
    pub reason: String,
}

#[derive(Debug, Clone)]
/// `updated_at`: feed `latestRoundData.updatedAt` (seconds). `None` → BLOCK.
/// `now_seconds`: caller clock. `max_age_seconds`: caller policy (`>= 0`).
pub struct IsStaleInput {
    pub updated_at: Option<i64>,
    pub now_seconds: i64,
    pub max_age_seconds: i64,
}

/// Pure freshness check — no RPC. Fail closed on missing, unparseable, or future timestamp.
/// Official field: Data Feed `latestRoundData.updatedAt` (uint80 seconds).
///
/// # Returns
/// `Allow` iff `0 <= now - updatedAt <= max_age_seconds`.
pub fn is_stale(input: IsStaleInput) -> IsStaleResult {
    if input.max_age_seconds < 0 {
        return IsStaleResult {
            decision: Decision::Block,
            age_seconds: None,
            reason: "maxAgeSeconds must be >= 0 — BLOCK".to_string(),
        };
    }

    let updated_at = match input.updated_at {
        Some(t) if t >= 0 => t,
        _ => {
            return IsStaleResult {
                decision: Decision::Block,
                age_seconds: None,
                reason: "missing or unparseable updatedAt — BLOCK (fail closed)".to_string(),
            };
        }
    };

    let age_seconds = match input.now_seconds.checked_sub(updated_at) {
        Some(age) => age,
        None => {
            return IsStaleResult {
                decision: Decision::Block,
                age_seconds: None,
                reason: "timestamp arithmetic overflow — BLOCK (fail closed)".to_string(),
            };
        }
    };

    if age_seconds < 0 {
        return IsStaleResult {
            decision: Decision::Block,
            age_seconds: Some(age_seconds),
            reason: format!(
                "not-yet-valid: updatedAt {} is in the future (now {}) — BLOCK",
                updated_at, input.now_seconds
            ),
        };
    }

    if age_seconds > input.max_age_seconds {
        return IsStaleResult {
            decision: Decision::Block,
            age_seconds: Some(age_seconds),
            reason: format!(
                "stale: age {}s > maxAge {}s — BLOCK",
                age_seconds, input.max_age_seconds
            ),
        };
    }

    IsStaleResult {
        decision: Decision::Allow,
        age_seconds: Some(age_seconds),
        reason: format!(
            "fresh: age {}s <= maxAge {}s — ALLOW",
            age_seconds, input.max_age_seconds
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fresh_age_less_than_max_age() {
        let res = is_stale(IsStaleInput {
            updated_at: Some(100),
            now_seconds: 150,
            max_age_seconds: 60,
        });
        assert_eq!(res.decision, Decision::Allow);
        assert_eq!(res.age_seconds, Some(50));
    }

    #[test]
    fn test_stale_age_greater_than_max_age() {
        let res = is_stale(IsStaleInput {
            updated_at: Some(100),
            now_seconds: 200,
            max_age_seconds: 60,
        });
        assert_eq!(res.decision, Decision::Block);
        assert_eq!(res.age_seconds, Some(100));
        assert!(res.reason.contains("stale"));
    }

    #[test]
    fn test_future_updated_at_blocks() {
        let res = is_stale(IsStaleInput {
            updated_at: Some(250),
            now_seconds: 200,
            max_age_seconds: 60,
        });
        assert_eq!(res.decision, Decision::Block);
        assert!(res.reason.contains("not-yet-valid"));
    }

    #[test]
    fn test_missing_updated_at_blocks() {
        let res = is_stale(IsStaleInput {
            updated_at: None,
            now_seconds: 200,
            max_age_seconds: 60,
        });
        assert_eq!(res.decision, Decision::Block);
        assert_eq!(res.age_seconds, None);
    }

    #[test]
    fn test_negative_max_age_blocks() {
        let res = is_stale(IsStaleInput {
            updated_at: Some(100),
            now_seconds: 200,
            max_age_seconds: -1,
        });
        assert_eq!(res.decision, Decision::Block);
    }

    #[test]
    fn test_timestamp_overflow_blocks_fail_closed() {
        let res = is_stale(IsStaleInput {
            updated_at: Some(i64::MAX),
            now_seconds: i64::MIN,
            max_age_seconds: 60,
        });
        assert_eq!(res.decision, Decision::Block);
        assert!(res.reason.contains("overflow"));
    }
}
