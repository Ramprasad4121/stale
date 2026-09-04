//! Honeypot / transferability probe.
//!
//! Simulates `transfer(DEAD, amount)` via `eth_call(from = holder)`.
//! Success proves *transferability*, not *fair taxation*: fee-on-transfer,
//! sell-tax, and buy-ok/sell-block tokens can still pass. Treat ALLOW as
//! "not an outright honeypot" and compose with tax measurement for pricing.
//! Any revert/transport error → BLOCK (fail closed).

use crate::abi::{decode_bool, encode_address_param, encode_u256_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const TRANSFER_SELECTOR: &str = "0xa9059cbb";
pub const DEAD_ADDRESS: &str = "0x000000000000000000000000000000000000dEaD";

/// Probe `token.transfer(dead, amount)` from `holder`. `amount == 0`
/// is rejected (vacuous simulation). See module caveats.
pub async fn check_token_tax(
    client: &dyn EvmRpcClient,
    token: &str,
    holder: &str,
    amount: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(token) {
        return GuardrailResult::block(format!("invalid token address {} — BLOCK", token));
    }
    if !is_valid_eth_address(holder) {
        return GuardrailResult::block(format!("invalid holder address {} — BLOCK", holder));
    }
    if amount == 0 {
        return GuardrailResult::block("invalid amount 0 — BLOCK");
    }

    let dead_enc = match encode_address_param(DEAD_ADDRESS) {
        Ok(e) => e,
        Err(e) => return GuardrailResult::block(format!("{} — BLOCK (fail closed)", e)),
    };
    let calldata = format!(
        "{}{}{}",
        TRANSFER_SELECTOR,
        dead_enc,
        encode_u256_param(amount)
    );

    match client.call_from(holder, token, &calldata).await {
        Ok(hex_data) => match decode_bool(&hex_data) {
            Ok(true) => GuardrailResult::allow(
                "token transfer simulation succeeded — token is transferable",
            ),
            Ok(false) => GuardrailResult::block(format!(
                "HONEYPOT/FEE DETECTED: token {} returned false on transfer simulation from holder {} (fee-on-transfer or blocklisted) — BLOCK",
                token, holder
            )),
            Err(e) => GuardrailResult::block(format!(
                "failed to decode transfer simulation response for {} — BLOCK (fail closed): {}",
                token, e
            )),
        },
        Err(e) => {
            let lower = e.to_lowercase();
            if lower.contains("exceeds balance") || lower.contains("insufficient balance") {
                GuardrailResult::block(format!(
                    "cannot simulate transfer: holder {} does not hold required {} of token {} — BLOCK",
                    holder, amount, token
                ))
            } else if lower.contains("revert") || lower.contains("execution reverted") {
                GuardrailResult::block(format!(
                    "HONEYPOT DETECTED: token {} reverted on transfer simulation from holder {}. This token cannot be sold or transferred. — BLOCK: {}",
                    token, holder, e
                ))
            } else {
                GuardrailResult::block(format!(
                    "failed to simulate token transfer for {} — BLOCK (fail closed): {}",
                    token, e
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_transfer_succeeds_allowed() {
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(format!("0x{:0>64x}", 1u64)))),
            ..Default::default()
        };

        let res = check_token_tax(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            1000,
        )
        .await;

        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_transfer_reverts_honeypot_blocked() {
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| {
                Err("execution reverted: transfer disabled".to_string())
            })),
            ..Default::default()
        };

        let res = check_token_tax(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            1000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("HONEYPOT DETECTED"));
    }

    #[tokio::test]
    async fn test_transfer_false_return_blocked() {
        // Fee-on-transfer / blocklisted tokens return `false` instead of
        // reverting — must BLOCK, not ALLOW.
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(format!("0x{:0>64x}", 0u64)))),
            ..Default::default()
        };

        let res = check_token_tax(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            1000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("returned false"));
    }

    #[tokio::test]
    async fn test_transfer_nonzero_garbage_blocked() {
        // Strict bool: only 0/1 are valid. A `2` must BLOCK, never ALLOW.
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(format!("0x{:0>64x}", 2u64)))),
            ..Default::default()
        };

        let res = check_token_tax(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            1000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("fail closed"));
    }

    #[tokio::test]
    async fn test_transfer_malformed_response_blocked() {
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok("0xdead".to_string()))),
            ..Default::default()
        };

        let res = check_token_tax(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            1000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("fail closed"));
    }
}
