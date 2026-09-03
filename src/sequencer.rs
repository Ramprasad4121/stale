use crate::abi::decode_round_data;
use crate::feeds::get_sequencer_feed;
use crate::rpc::EvmRpcClient;

pub const SEQUENCER_SELECTOR: &str = "0xfeaf968c";
pub const GRACE_PERIOD_SECONDS: u64 = 3600;

/// Validates L2 sequencer status using the official Chainlink Uptime Feed.
/// Returns Ok(None) if L2 is up and past grace period, or if chain has no sequencer feed.
/// Returns Ok(Some(reason)) or Err(reason) if sequencer is down, in grace period, or query failed.
pub async fn check_sequencer(
    chain_id: u64,
    client: &dyn EvmRpcClient,
    now_seconds: u64,
) -> Option<String> {
    let feed_address = match get_sequencer_feed(chain_id) {
        Some(addr) => addr,
        None => return None, // Not an L2 or no feed configured
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
}
