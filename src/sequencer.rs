//! L2 sequencer liveness via Chainlink uptime feeds.
//!
//! Returns `None` (pass) only when the sequencer reports up (`answer == 0`)
//! past the restart grace period ([`GRACE_PERIOD_SECONDS`]). Down, unknown
//! status, incomplete round, missing data, future `startedAt`, grace-period
//! restart, decode failure, and transport failure all yield `Some(reason)`
//! — the caller maps every `Some` to BLOCK.

use crate::abi::decode_round_data;
use crate::feeds::{get_sequencer_feed, is_unconfigured_sequenced_l2};
use crate::rpc::EvmRpcClient;

pub const SEQUENCER_SELECTOR: &str = "0xfeaf968c";
/// Grace period after a sequencer restart during which reads still BLOCK.
pub const GRACE_PERIOD_SECONDS: u64 = 3600;

/// Validates L2 sequencer status using the official Chainlink Uptime Feed.
/// Returns Ok(None) if L2 is up and past grace period, or if chain has no sequencer feed.
/// Returns Ok(Some(reason)) or Err(reason) if sequencer is down, in grace period, or query failed.
///
/// A sequenced L2 with no configured feed (Blast, Linea, Arbitrum Nova)
/// returns `Some` — loud unprotected-sequencer BLOCK, never a silent pass.
///
/// # Params
/// - `chain_id`: EVM chain id (non-L2 / unknown → `None`, no check).
/// - `now_seconds`: caller clock (Unix seconds, `u64`).
pub async fn check_sequencer(
    chain_id: u64,
    client: &dyn EvmRpcClient,
    now_seconds: u64,
) -> Option<String> {
    let feed_address = match get_sequencer_feed(chain_id) {
        Some(addr) => addr,
        None => {
            if is_unconfigured_sequenced_l2(chain_id) {
                return Some(format!(
                    "chain {} has a centralized sequencer but no uptime feed is configured — BLOCK (fail closed: add the feed, do not assume liveness)",
                    chain_id
                ));
            }
            return None; // Not an L2 or no feed configured
        }
    };

    match client.call(feed_address, SEQUENCER_SELECTOR).await {
        Ok(hex_data) => match decode_round_data(&hex_data) {
            Ok((round_id, answer, started_at, updated_at, answered_in_round)) => {
                if answered_in_round < round_id {
                    return Some(format!(
                        "sequencer feed incomplete round on chain {} — BLOCK",
                        chain_id
                    ));
                }
                if updated_at == 0 {
                    return Some(format!(
                        "sequencer feed has no data (updatedAt 0) on chain {} — BLOCK",
                        chain_id
                    ));
                }
                if answer == 1 {
                    return Some(format!(
                        "L2 Sequencer is DOWN on chain {} — BLOCK",
                        chain_id
                    ));
                }

                if answer != 0 {
                    return Some(format!(
                        "unexpected sequencer status {} on chain {} — BLOCK (fail closed)",
                        answer, chain_id
                    ));
                }

                {
                    let started_at_u64 = if started_at > u64::MAX as u128 {
                        return Some(format!(
                            "sequencer startedAt unrepresentable on chain {} — BLOCK",
                            chain_id
                        ));
                    } else {
                        started_at as u64
                    };
                    if now_seconds < started_at_u64 {
                        return Some(format!(
                            "L2 Sequencer startedAt is in future on chain {} — BLOCK",
                            chain_id
                        ));
                    }
                    let time_since_up = now_seconds - started_at_u64;
                    if time_since_up < GRACE_PERIOD_SECONDS {
                        return Some(format!(
                            "L2 Sequencer is in grace period ({}s < {}s) on chain {} — BLOCK",
                            time_since_up, GRACE_PERIOD_SECONDS, chain_id
                        ));
                    }
                }
                None
            }
            Err(e) => Some(format!(
                "failed to decode sequencer round data on chain {} — BLOCK (fail closed): {}",
                chain_id, e
            )),
        },
        Err(e) => Some(format!(
            "failed to read sequencer feed on chain {} — BLOCK (fail closed): {}",
            chain_id, e
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::feeds::ARBITRUM_CHAIN_ID;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_sequencer_up_past_grace_period() {
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 0u64, 1000u64, 1000u64, 1u64
        );
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(round_data_hex.clone()))),
            ..Default::default()
        };

        // now = 1000 + 4000 = 5000 (past 3600s grace period)
        let res = check_sequencer(ARBITRUM_CHAIN_ID, &mock, 5000).await;
        assert_eq!(res, None);
    }

    #[tokio::test]
    async fn test_sequencer_down() {
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 1u64, 1000u64, 1000u64, 1u64
        );
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(round_data_hex.clone()))),
            ..Default::default()
        };

        let res = check_sequencer(ARBITRUM_CHAIN_ID, &mock, 5000).await;
        assert!(res.is_some());
        assert!(res.unwrap().contains("L2 Sequencer is DOWN"));
    }

    #[tokio::test]
    async fn test_sequencer_in_grace_period() {
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 0u64, 1000u64, 1000u64, 1u64
        );
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(round_data_hex.clone()))),
            ..Default::default()
        };

        // now = 1000 + 500 = 1500 (< 3600s grace period)
        let res = check_sequencer(ARBITRUM_CHAIN_ID, &mock, 1500).await;
        assert!(res.is_some());
        assert!(res.unwrap().contains("grace period"));
    }

    #[tokio::test]
    async fn test_sequencer_unexpected_answer_blocks_fail_closed() {
        let round_data_hex = format!(
            "0x{:0>64x}{:0>64x}{:0>64x}{:0>64x}{:0>64x}",
            1u64, 2u64, 1000u64, 1000u64, 1u64
        );
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(round_data_hex.clone()))),
            ..Default::default()
        };

        let res = check_sequencer(ARBITRUM_CHAIN_ID, &mock, 5000).await;
        assert!(res.is_some());
        assert!(res.unwrap().contains("fail closed"));
    }

    #[tokio::test]
    async fn test_unconfigured_sequenced_l2_blocks_loudly() {
        use crate::feeds::{ARBITRUM_NOVA_CHAIN_ID, BLAST_CHAIN_ID, LINEA_CHAIN_ID};
        // No RPC interaction at all: the BLOCK fires before any call.
        let mock = MockRpcClient::default();
        for chain in [BLAST_CHAIN_ID, LINEA_CHAIN_ID, ARBITRUM_NOVA_CHAIN_ID] {
            let res = check_sequencer(chain, &mock, 5000).await;
            assert!(res.is_some(), "chain {} must not silently pass", chain);
            assert!(
                res.unwrap().contains("no uptime feed is configured"),
                "chain {}",
                chain
            );
        }
    }

    #[tokio::test]
    async fn test_non_sequenced_chain_still_passes() {
        use crate::feeds::{MAINNET_CHAIN_ID, POLYGON_CHAIN_ID};
        let mock = MockRpcClient::default();
        assert_eq!(check_sequencer(MAINNET_CHAIN_ID, &mock, 5000).await, None);
        assert_eq!(check_sequencer(POLYGON_CHAIN_ID, &mock, 5000).await, None);
    }
}
