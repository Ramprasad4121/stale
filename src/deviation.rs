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
/// any RPC/decode failure → BLOCK. The verdict uses exact integer math
/// (basis points, checked `u128`); `f64` appears only in display strings.
/// Thresholds below 1 bps behave as 1 bps; thresholds above 100% are
/// rejected (a >100% threshold would disable the guard).
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
    if !max_deviation_percent.is_finite()
        || max_deviation_percent <= 0.0
        || max_deviation_percent > 100.0
    {
        return GuardrailResult::block(
            "invalid maxDeviationPercent (must be finite, > 0, ≤ 100) — BLOCK",
        );
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

    // Exact integer deviation — f64 never touches the verdict.
    //
    // Scale both answers to common decimals D, then BLOCK iff
    //   |A - B| * 10_000 > ceil(max% * 100) * (A + B)
    // (deviation in basis points vs threshold in basis points). Every op
    // is checked; any overflow BLOCKs (reachable only at absurd ≥1e32
    // magnitudes — availability-only, never a false ALLOW). Thresholds
    // below 1 bps behave as 1 bps (documented floor); `ceil` keeps the
    // integer threshold on the strict (safe) side of the caller's f64.
    let common_decimals = dec_a.max(dec_b);
    let scaled_a = match scale_answer(ans_a, dec_a, common_decimals) {
        Some(v) => v,
        None => {
            return GuardrailResult::block(
                "feedA magnitude exceeds exact-comparison range — BLOCK (fail closed)",
            )
        }
    };
    let scaled_b = match scale_answer(ans_b, dec_b, common_decimals) {
        Some(v) => v,
        None => {
            return GuardrailResult::block(
                "feedB magnitude exceeds exact-comparison range — BLOCK (fail closed)",
            )
        }
    };
    let spread = scaled_a.abs_diff(scaled_b);
    let total = match scaled_a.checked_add(scaled_b) {
        Some(t) => t,
        None => return GuardrailResult::block("deviation sum overflows — BLOCK (fail closed)"),
    };
    let threshold_bps = (max_deviation_percent * 100.0).ceil() as u64;
    let left = match spread.checked_mul(10_000) {
        Some(v) => v,
        None => {
            return GuardrailResult::block(
                "deviation spread overflows basis-point scaling — BLOCK (fail closed)",
            )
        }
    };
    let right = match (threshold_bps as u128).checked_mul(total) {
        Some(v) => v,
        None => {
            return GuardrailResult::block(
                "deviation threshold scaling overflows — BLOCK (fail closed)",
            )
        }
    };

    // Display-only approximation (never the verdict).
    let deviation_percent = (spread as f64) / (total as f64) * 100.0;

    if left > right {
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

/// Scale a positive `answer` from `decimals` to `target` (≥ decimals):
/// `answer * 10^(target - decimals)`. `None` on overflow. Answers are
/// pre-validated `> 0`, so the `u128` conversion is infallible.
fn scale_answer(answer: i128, decimals: u32, target: u32) -> Option<u128> {
    let base = u128::try_from(answer).ok()?;
    let factor = 10u128.checked_pow(target - decimals)?;
    base.checked_mul(factor)
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

    fn round_hex(answer: u128, updated: u64) -> String {
        format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, answer, 1000u64, updated, 1u64
        )
    }

    #[tokio::test]
    async fn test_threshold_above_100_rejected() {
        let mock = MockRpcClient::default();
        let res = check_price_deviation(
            &mock,
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            101.0,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("maxDeviationPercent"));
    }

    #[tokio::test]
    async fn test_large_magnitude_exact_verdict() {
        // 2^60-scale answers: f64 would blur these, integers must not.
        // A = 2^60, B = 2^60 + 2^60/20 (5% apart) → BLOCK at 2% threshold.
        let base: u128 = 1u128 << 60;
        let bumped: u128 = base + (base / 20);
        let dec_hex = format!("0x{:0>64x}", 8u64);
        let round_a = round_hex(base, 1000);
        let round_b = round_hex(bumped, 1000);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |addr: &str, data: &str| {
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
    async fn test_mismatched_decimals_exact() {
        // Same $2500 price, 18 vs 8 decimals → 0 bps → ALLOW.
        let price_18: u128 = 2500 * 10u128.pow(18);
        let price_8: u128 = 2500 * 10u128.pow(8);
        let round_a = round_hex(price_18, 1000);
        let round_b = round_hex(price_8, 1000);
        let dec_18 = format!("0x{:0>64x}", 18u64);
        let dec_8 = format!("0x{:0>64x}", 8u64);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |addr: &str, data: &str| {
                if data == LATEST_ROUND_DATA_SELECTOR {
                    if addr.contains("1111") {
                        Ok(round_a.clone())
                    } else {
                        Ok(round_b.clone())
                    }
                } else if data == DECIMALS_SELECTOR {
                    if addr.contains("1111") {
                        Ok(dec_18.clone())
                    } else {
                        Ok(dec_8.clone())
                    }
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
}
