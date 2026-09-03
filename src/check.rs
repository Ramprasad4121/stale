use crate::abi::{decode_round_data, decode_word_u128};
use crate::addressbook::is_valid_eth_address;
use crate::feeds::lookup_feed;
use crate::is_stale::{is_stale, IsStaleInput};
use crate::quote::{quote_from_feed, QuoteInput};
use crate::rpc::EvmRpcClient;
use crate::sequencer::check_sequencer;
use crate::types::Decision;
use serde::{Deserialize, Serialize};

pub const LATEST_ROUND_DATA_SELECTOR: &str = "0xfeaf968c";
pub const DECIMALS_SELECTOR: &str = "0x313ce567";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckPriceResult {
    pub decision: Decision,
    pub reason: String,
    pub feed: String,
    pub answer: String,
    pub price_usd: Option<f64>,
    pub amount_eth: Option<f64>,
    pub quote_usd: Option<f64>,
    pub updated_at: String,
    pub age_seconds: Option<i64>,
    pub max_age_seconds: i64,
    pub now: i64,
    pub allow_execute: bool,
}

#[derive(Debug, Clone)]
pub struct CheckPriceInput<'a> {
    pub feed: &'a str,
    pub max_age_seconds: i64,
    pub amount_eth: Option<f64>,
    pub now_seconds: Option<i64>,
}

fn block_result(
    feed: &str,
    reason: String,
    max_age_seconds: i64,
    now: i64,
    amount_eth: Option<f64>,
) -> CheckPriceResult {
    CheckPriceResult {
        decision: Decision::Block,
        reason,
        feed: feed.to_string(),
        answer: "0".to_string(),
        price_usd: None,
        amount_eth,
        quote_usd: None,
        updated_at: "0".to_string(),
        age_seconds: None,
        max_age_seconds,
        now,
        allow_execute: false,
    }
}

pub async fn check_price(
    client: &dyn EvmRpcClient,
    input: CheckPriceInput<'_>,
) -> CheckPriceResult {
    let now = input
        .now_seconds
        .unwrap_or_else(|| chrono::Utc::now().timestamp());

    if now < 0 {
        return block_result(
            input.feed,
            "invalid now timestamp (negative) — BLOCK".to_string(),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    if !is_valid_eth_address(input.feed) {
        return block_result(
            input.feed,
            format!("invalid feed address {} — BLOCK", input.feed),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    if input.max_age_seconds < 0 {
        return block_result(
            input.feed,
            "maxAgeSeconds must be >= 0 — BLOCK".to_string(),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    // Check if feed is allowlisted in registry
    let feed_entry = match lookup_feed(input.feed) {
        Some(e) => e,
        None => {
            return block_result(
                input.feed,
                format!("unknown feed {} (not allowlisted) — BLOCK", input.feed),
                input.max_age_seconds,
                now,
                input.amount_eth,
            );
        }
    };

    // If on an L2 chain, check sequencer status
    if let Some(seq_block) = check_sequencer(feed_entry.chain_id, client, now as u64).await {
        return block_result(
            input.feed,
            seq_block,
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    // Query round data and decimals
    let (round_res, dec_res) = tokio::join!(
        client.call(input.feed, LATEST_ROUND_DATA_SELECTOR),
        client.call(input.feed, DECIMALS_SELECTOR),
    );

    let hex_round = match round_res {
        Ok(h) => h,
        Err(e) => {
            return block_result(
                input.feed,
                format!(
                    "failed to read latestRoundData() from {} — BLOCK (fail closed): {}",
                    input.feed, e
                ),
                input.max_age_seconds,
                now,
                input.amount_eth,
            );
        }
    };

    let hex_dec = match dec_res {
        Ok(h) => h,
        Err(e) => {
            return block_result(
                input.feed,
                format!(
                    "failed to read decimals() from {} — BLOCK (fail closed): {}",
                    input.feed, e
                ),
                input.max_age_seconds,
                now,
                input.amount_eth,
            );
        }
    };

    let (round_id, answer, _started_at, updated_at, answered_in_round) =
        match decode_round_data(&hex_round) {
            Ok(d) => d,
            Err(e) => {
                return block_result(
                    input.feed,
                    format!(
                        "failed to decode latestRoundData() from {}: {}",
                        input.feed, e
                    ),
                    input.max_age_seconds,
                    now,
                    input.amount_eth,
                );
            }
        };

    let decimals = match decode_word_u128(&hex_dec, 0) {
        Ok(d) if d <= 18 => d as u8,
        _ => {
            return block_result(
                input.feed,
                format!("invalid decimals response from {}", input.feed),
                input.max_age_seconds,
                now,
                input.amount_eth,
            );
        }
    };

    if updated_at == 0 {
        return block_result(
            input.feed,
            format!("updatedAt is 0 (no data) from {} — BLOCK", input.feed),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    if answer <= 0 {
        return block_result(
            input.feed,
            format!(
                "answer is {} (invalid price) from {} — BLOCK",
                answer, input.feed
            ),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    if answered_in_round < round_id {
        return block_result(
            input.feed,
            format!(
                "incomplete round: answeredInRound {} < roundId {} (unanswered round) from {} — BLOCK",
                answered_in_round, round_id, input.feed
            ),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }

    // updatedAt is uint80 on-chain; reject values unrepresentable as i64
    // instead of silently wrapping via `as` cast.
    if updated_at > i64::MAX as u128 {
        return block_result(
            input.feed,
            format!(
                "updatedAt {} exceeds i64::MAX (unrepresentable) from {} — BLOCK",
                updated_at, input.feed
            ),
            input.max_age_seconds,
            now,
            input.amount_eth,
        );
    }
    let updated_at_i64 = updated_at as i64;

    let stale_check = is_stale(IsStaleInput {
        updated_at: Some(updated_at_i64),
        now_seconds: now,
        max_age_seconds: input.max_age_seconds,
    });

    let (price_usd, quote_usd) = match quote_from_feed(QuoteInput {
        answer,
        decimals,
        amount: input.amount_eth,
    }) {
        Ok(q) => (Some(q.price_usd), q.quote_usd),
        Err(e) => {
            return CheckPriceResult {
                decision: Decision::Block,
                reason: format!("quote failed: {} — BLOCK", e),
                feed: input.feed.to_string(),
                answer: answer.to_string(),
                price_usd: None,
                amount_eth: input.amount_eth,
                quote_usd: None,
                updated_at: updated_at.to_string(),
                age_seconds: stale_check.age_seconds,
                max_age_seconds: input.max_age_seconds,
                now,
                allow_execute: false,
            };
        }
    };

    CheckPriceResult {
        decision: stale_check.decision,
        reason: stale_check.reason,
        feed: input.feed.to_string(),
        answer: answer.to_string(),
        price_usd,
        amount_eth: input.amount_eth,
        quote_usd,
        updated_at: updated_at.to_string(),
        age_seconds: stale_check.age_seconds,
        max_age_seconds: input.max_age_seconds,
        now,
        allow_execute: stale_check.decision == Decision::Allow,
    }
}

pub async fn check_prices(
    client: &dyn EvmRpcClient,
    feeds: Vec<CheckPriceInput<'_>>,
) -> Vec<CheckPriceResult> {
    let mut results = Vec::new();
    for f in feeds {
        results.push(check_price(client, f).await);
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feeds::DEFAULT_FEED;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_check_price_fresh_allowed() {
        let now = 1700000050;
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            10u64, 2500_00000000u64, 1700000000u64, 1700000040u64, 10u64
        );
        let dec_hex = format!("0x{:0>64x}", 8u64);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, data| {
                if data == LATEST_ROUND_DATA_SELECTOR {
                    Ok(round_data_hex.clone())
                } else if data == DECIMALS_SELECTOR {
                    Ok(dec_hex.clone())
                } else {
                    Err("unknown".to_string())
                }
            })),
            ..Default::default()
        };

        let res = check_price(
            &mock,
            CheckPriceInput {
                feed: DEFAULT_FEED,
                max_age_seconds: 60,
                amount_eth: Some(1.0),
                now_seconds: Some(now),
            },
        )
        .await;

        assert_eq!(res.decision, Decision::Allow);
        assert!(res.allow_execute);
        assert!((res.price_usd.unwrap() - 2500.0).abs() < 1e-3);
        assert_eq!(res.age_seconds, Some(10));
    }

    #[tokio::test]
    async fn test_check_price_stale_blocked() {
        let now = 1700000200; // 160s old
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            10u64, 2500_00000000u64, 1700000000u64, 1700000040u64, 10u64
        );
        let dec_hex = format!("0x{:0>64x}", 8u64);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, data| {
                if data == LATEST_ROUND_DATA_SELECTOR {
                    Ok(round_data_hex.clone())
                } else if data == DECIMALS_SELECTOR {
                    Ok(dec_hex.clone())
                } else {
                    Err("unknown".to_string())
                }
            })),
            ..Default::default()
        };

        let res = check_price(
            &mock,
            CheckPriceInput {
                feed: DEFAULT_FEED,
                max_age_seconds: 60,
                amount_eth: None,
                now_seconds: Some(now),
            },
        )
        .await;

        assert_eq!(res.decision, Decision::Block);
        assert!(!res.allow_execute);
        assert!(res.reason.contains("stale"));
    }

    #[tokio::test]
    async fn test_check_price_updated_at_overflow_blocked() {
        let now = 1700000200;
        let huge_updated_at: u128 = 1u128 << 70; // exceeds i64::MAX
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            10u64, 2500_00000000u64, 1700000000u64, huge_updated_at, 10u64
        );
        let dec_hex = format!("0x{:0>64x}", 8u64);

        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, data| {
                if data == LATEST_ROUND_DATA_SELECTOR {
                    Ok(round_data_hex.clone())
                } else if data == DECIMALS_SELECTOR {
                    Ok(dec_hex.clone())
                } else {
                    Err("unknown".to_string())
                }
            })),
            ..Default::default()
        };

        let res = check_price(
            &mock,
            CheckPriceInput {
                feed: DEFAULT_FEED,
                max_age_seconds: 60,
                amount_eth: None,
                now_seconds: Some(now),
            },
        )
        .await;

        assert_eq!(res.decision, Decision::Block);
        assert!(!res.allow_execute);
        assert!(res.reason.contains("unrepresentable"));
    }

    #[tokio::test]
    async fn test_check_price_unknown_feed_blocked() {
        let mock = MockRpcClient::default();
        let res = check_price(
            &mock,
            CheckPriceInput {
                feed: "0x0000000000000000000000000000000000000000",
                max_age_seconds: 60,
                amount_eth: None,
                now_seconds: Some(1000),
            },
        )
        .await;

        assert_eq!(res.decision, Decision::Block);
        assert!(res.reason.contains("not allowlisted"));
    }
}
