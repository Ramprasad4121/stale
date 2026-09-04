//! OFAC compliance guard via the Chainlink sanctions oracle.
//!
//! Any sanctioned hit, RPC failure, or decode failure → BLOCK. This is a
//! point-in-time read, not legal advice; record the verdict in the audit log.
//!
//! # Chain binding
//! The caller-supplied `chain_id` is verified against the transport's
//! `eth_chainId` before anything else. A caller connected to another chain
//! but passing `1` would otherwise query whatever contract lives at the
//! oracle address there and read a clean-looking ALLOW.

use crate::abi::{decode_bool, encode_address_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const SANCTIONS_ORACLE: &str = "0x40C57923924B5c5c5455c48D93317139ADDaC8fb";
/// Official Chainalysis / Chainlink Oracle isSanctioned(address) function selector
pub const IS_SANCTIONED_SELECTOR: &str = "0xdf592f7d";
/// The sanctions oracle is deployed on Ethereum mainnet only.
pub const SANCTIONS_CHAIN_ID: u64 = 1;

/// BLOCK if `address` is sanctioned per [`SANCTIONS_ORACLE`].
/// Fail closed on every error path. `chain_id` declares the expected
/// chain AND is verified against the RPC's `eth_chainId`: a mismatch
/// BLOCKs rather than querying an address that is not the oracle on the
/// connected chain. `chain_id != SANCTIONS_CHAIN_ID` BLOCKs without any
/// query.
pub async fn check_sanctioned(
    client: &dyn EvmRpcClient,
    chain_id: u64,
    address: &str,
) -> GuardrailResult {
    let actual_chain = match client.get_chain_id().await {
        Ok(c) => c,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to verify chain id for sanctions check — BLOCK (fail closed): {}",
                e
            ));
        }
    };
    if actual_chain != chain_id {
        return GuardrailResult::block(format!(
            "sanctions chain_id argument {} does not match connected chain {} — BLOCK (fail closed)",
            chain_id, actual_chain
        ));
    }
    if chain_id != SANCTIONS_CHAIN_ID {
        return GuardrailResult::block(format!(
            "sanctions oracle is only deployed on chain {} (queried chain {}) — BLOCK",
            SANCTIONS_CHAIN_ID, chain_id
        ));
    }
    if !is_valid_eth_address(address) {
        return GuardrailResult::block(format!("invalid address {} — BLOCK", address));
    }

    let encoded = match encode_address_param(address) {
        Ok(e) => e,
        Err(e) => return GuardrailResult::block(format!("{} — BLOCK (fail closed)", e)),
    };
    let calldata = format!("{}{}", IS_SANCTIONED_SELECTOR, encoded);

    let is_sanctioned = match client.call(SANCTIONS_ORACLE, &calldata).await {
        Ok(hex_data) => match decode_bool(&hex_data) {
            Ok(b) => b,
            Err(e) => {
                return GuardrailResult::block(format!(
                    "failed to decode sanctions oracle response — BLOCK (fail closed): {}",
                    e
                ));
            }
        },
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to query sanctions oracle for {} — BLOCK (fail closed): {}",
                address, e
            ));
        }
    };

    if is_sanctioned {
        GuardrailResult::block(format!(
            "COMPLIANCE VIOLATION: address {} is sanctioned — BLOCK",
            address
        ))
    } else {
        GuardrailResult::allow("address is compliant (not on sanctions list)")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_address_sanctioned_blocked() {
        let hex = format!("0x{:0>64x}", 1u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            chain_id: Some(1),
            ..Default::default()
        };

        let res = check_sanctioned(&mock, 1, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("COMPLIANCE VIOLATION"));
    }

    #[tokio::test]
    async fn test_address_clean_allowed() {
        let hex = format!("0x{:0>64x}", 0u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            chain_id: Some(1),
            ..Default::default()
        };

        let res = check_sanctioned(&mock, 1, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(res.allow_execute);
        assert!(res.reason.contains("compliant"));
    }

    #[tokio::test]
    async fn test_wrong_chain_blocked_without_oracle_query() {
        // Mock would ALLOW (returns false), but the chain gate must fire
        // first — never query a non-oracle address on another chain.
        let hex = format!("0x{:0>64x}", 0u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            chain_id: Some(10),
            ..Default::default()
        };

        let res = check_sanctioned(&mock, 10, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("only deployed on chain 1"));
    }

    #[tokio::test]
    async fn test_chain_id_argument_mismatch_blocks() {
        // Caller claims mainnet but the transport is on Optimism: BLOCK
        // instead of querying a non-oracle address.
        let hex = format!("0x{:0>64x}", 0u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            chain_id: Some(10),
            ..Default::default()
        };

        let res = check_sanctioned(&mock, 1, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("does not match connected chain"));
    }

    #[tokio::test]
    async fn test_chain_id_unverifiable_blocks_fail_closed() {
        let hex = format!("0x{:0>64x}", 0u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            ..Default::default()
        };

        let res = check_sanctioned(&mock, 1, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("fail closed"));
    }
}
