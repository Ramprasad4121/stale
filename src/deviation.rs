//! Multi-oracle deviation guard: two independent feeds, one asset.
//!
//! Blocks when feeds disagree beyond `max_deviation_percent` — the
//! standard defense against single-oracle (flash-loan) manipulation.
//! Compares **normalized** prices (`answer / 10^decimals`), so feeds with
//! different decimals are comparable. Rejects incomplete rounds and
//! `updatedAt == 0` on either feed before comparing.
//!
//! # Staleness caveat
//! This guard checks round *completeness*, not *freshness*: pass an
//! explicit `maxAge` policy via `is_stale()` / `check_price()` alongside
//! it. A fresh-vs-stale comparison can otherwise agree today
//! and diverge tomorrow.

use crate::abi::{decode_round_data, decode_word_u128};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;
use serde_json::json;

pub const LATEST_ROUND_DATA_SELECTOR: &str = "0xfeaf968c";
pub const DECIMALS_SELECTOR: &str = "0x313ce567";

/// Compare two feeds; BLOCK if `|a-b|/avg*100 > max_deviation_percent`.
/// Identical feeds, invalid/identical addresses, non-positive answers, or
/// any RPC/decode failure → BLOCK. Deviation is `f64` display math — the
/// *decision* threshold comparison is exact-direction (any `NaN`/non-finite
/// → BLOCK).
pub async fn check_price_deviation(
    client: &dyn EvmRpcClient,
    feed_a: &str,
    feed_b: &str,
    max_deviation_percent: f64,
) -> GuardrailResult {
    if !is_valid_eth_address(feed_a) {
        return GuardrailResult::block(format!("invalid feedA address {} — BLOCK", feed_a));
    }
    if !is_valid_eth_address(feed_b) {
        return GuardrailResult::block(format!("invalid feedB address {} — BLOCK", feed_b));
    }
    if !max_deviation_percent.is_finite() || max_deviation_percent <= 0.0 {
        return GuardrailResult::block("invalid maxDeviationPercent — BLOCK");
    }
    if feed_a.to_lowercase() == feed_b.to_lowercase() {
        return GuardrailResult::block(
            "self-comparison: feedA and feedB are identical (deviation always 0%) — BLOCK",
        );
    }

    let (round_a_res, dec_a_res, round_b_res, dec_b_res) = tokio::join!(
        client.call(feed_a, LATEST_ROUND_DATA_SELECTOR),
        client.call(feed_a, DECIMALS_SELECTOR),
        client.call(feed_b, LATEST_ROUND_DATA_SELECTOR),
        client.call(feed_b, DECIMALS_SELECTOR),
    );

    let round_a = match round_a_res.and_then(|h| decode_round_data(&h)) {
        Ok(d) => d,
        Err(e) => {
            return GuardrailResult::block(format!("failed to query feedA round data: {}", e))
        }
    };
    let dec_a = match dec_a_res.and_then(|h| decode_word_u128(&h, 0)) {
        Ok(d) => match u32::try_from(d) {
            Ok(v) => v,
            Err(_) => {
                return GuardrailResult::block("feedA decimals unrepresentable — BLOCK");
            }
        },
        Err(e) => return GuardrailResult::block(format!("failed to query feedA decimals: {}", e)),
    };

    let round_b = match round_b_res.and_then(|h| decode_round_data(&h)) {
        Ok(d) => d,
        Err(e) => {
            return GuardrailResult::block(format!("failed to query feedB round data: {}", e))
        }
    };
    let dec_b = match dec_b_res.and_then(|h| decode_word_u128(&h, 0)) {
        Ok(d) => match u32::try_from(d) {
            Ok(v) => v,
            Err(_) => {
                return GuardrailResult::block("feedB decimals unrepresentable — BLOCK");
            }
        },
        Err(e) => return GuardrailResult::block(format!("failed to query feedB decimals: {}", e)),
    };

    if dec_a > 36 || dec_b > 36 {
        return GuardrailResult::block("feed decimals exceed maximum supported 36 — BLOCK");
    }

    let ans_a = round_a.1;
    let ans_b = round_b.1;

    if ans_a <= 0 || ans_b <= 0 {
        return GuardrailResult::block("one or both feeds returned non-positive price — BLOCK");
    }

    // Never compare stale or incomplete rounds: a manipulated single-oracle
    // reading must not become the deviation baseline.
    for (label, round) in [("feedA", &round_a), ("feedB", &round_b)] {
        if round.3 == 0 {
            return GuardrailResult::block(format!("{} has no data (updatedAt 0) — BLOCK", label));
        }
        if round.4 < round.0 {
            return GuardrailResult::block(format!(
                "{} incomplete round: answeredInRound < roundId — BLOCK",
                label
            ));
        }
    }

    let price_a = (ans_a as f64) / 10_f64.powi(dec_a as i32);
    let price_b = (ans_b as f64) / 10_f64.powi(dec_b as i32);

    if !price_a.is_finite() || !price_b.is_finite() {
        return GuardrailResult::block("non-finite price computed from feeds — BLOCK");
    }

    let diff = (price_a - price_b).abs();
    let avg = (price_a + price_b) / 2.0;

    if avg <= 0.0 || !avg.is_finite() {
        return GuardrailResult::block("invalid average price — BLOCK");
    }

    let deviation_percent = (diff / avg) * 100.0;

    if !deviation_percent.is_finite() || deviation_percent > max_deviation_percent {
        GuardrailResult::block(format!(
            "ORACLE DEVIATION DANGER: feeds deviate by {:.2}% (max {:.2}%). Possible oracle manipulation. — BLOCK",
            deviation_percent, max_deviation_percent
        ))
        .with_metadata(json!({ "deviationPercent": deviation_percent }))
    } else {
        GuardrailResult::allow(format!(
            "feeds agree within {:.2}% deviation",
            deviation_percent
        ))
        .with_metadata(json!({ "deviationPercent": deviation_percent }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_deviation_within_threshold() {
        let round_a = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 2500_00000000u64, 1000u64, 1000u64, 1u64
        );
        let round_b = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 2505_00000000u64, 1000u64, 1000u64, 1u64
        );
        let dec_hex = format!("0x{:0>64x}", 8u64);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |addr, data| {
                if data == LATEST_ROUND_DATA_SELECTOR {
                    if addr.contains("1111") {
                        Ok(round_a.clone())
                    } else {
                        Ok(round_b.clone())
                    }
                } else if data == DECIMALS_SELECTOR {
                    Ok(dec_hex.clone())
                } else {
                    Err("unknown".to_string())
                }
            })),
            ..Default::default()
        };

        let res = check_price_deviation(
            &mock,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            2.0,
        )
        .await;

        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_deviation_exceeds_threshold() {
        let round_a = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 2500_00000000u64, 1000u64, 1000u64, 1u64
        );
        let round_b = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 2800_00000000u64, 1000u64, 1000u64, 1u64
        );
        let dec_hex = format!("0x{:0>64x}", 8u64);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |addr, data| {
                if data == LATEST_ROUND_DATA_SELECTOR {
                    if addr.contains("1111") {
                        Ok(round_a.clone())
                    } else {
                        Ok(round_b.clone())
                    }
                } else if data == DECIMALS_SELECTOR {
                    Ok(dec_hex.clone())
                } else {
                    Err("unknown".to_string())
                }
            })),
            ..Default::default()
        };

        let res = check_price_deviation(
            &mock,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            2.0,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("ORACLE DEVIATION DANGER"));
    }

    #[tokio::test]
    async fn test_self_comparison_blocked() {
        let mock = MockRpcClient::default();
        let res = check_price_deviation(
            &mock,
            "0x1111111111111111111111111111111111111111",
            "0x1111111111111111111111111111111111111111",
            2.0,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("self-comparison"));
    }
}
