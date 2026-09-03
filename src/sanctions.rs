//! OFAC compliance guard via the Chainlink sanctions oracle.
//!
//! Any sanctioned hit, RPC failure, or decode failure → BLOCK. This is a
//! point-in-time read, not legal advice; record the verdict in the audit log.

use crate::abi::{decode_bool, encode_address_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const SANCTIONS_ORACLE: &str = "0x40C57923924B5c5c5455c48D93317139ADDaC8fb";
/// Official Chainalysis / Chainlink Oracle isSanctioned(address) function selector
pub const IS_SANCTIONED_SELECTOR: &str = "0xdf592f7d";

/// BLOCK if `address` is sanctioned per [`SANCTIONS_ORACLE`].
/// Fail closed on every error path.
pub async fn check_sanctioned(client: &dyn EvmRpcClient, address: &str) -> GuardrailResult {
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
            ..Default::default()
        };

        let res = check_sanctioned(&mock, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("COMPLIANCE VIOLATION"));
    }

    #[tokio::test]
    async fn test_address_clean_allowed() {
        let hex = format!("0x{:0>64x}", 0u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            ..Default::default()
        };

        let res = check_sanctioned(&mock, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(res.allow_execute);
        assert!(res.reason.contains("compliant"));
    }
}
