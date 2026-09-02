use crate::abi::{encode_address_param, encode_u256_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const TRANSFER_SELECTOR: &str = "0xa9059cbb";
pub const DEAD_ADDRESS: &str = "0x000000000000000000000000000000000000dEaD";

pub async fn check_token_tax(
    client: &dyn EvmRpcClient,
    token: &str,
    _holder: &str,
    amount: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(token) {
        return GuardrailResult::block(format!("invalid token address {} — BLOCK", token));
    }
    if amount == 0 {
        return GuardrailResult::block("invalid amount 0 — BLOCK");
    }

    let calldata = format!(
        "{}{}{}",
        TRANSFER_SELECTOR,
        encode_address_param(DEAD_ADDRESS),
        encode_u256_param(amount)
    );

    match client.call(token, &calldata).await {
        Ok(_) => {
            GuardrailResult::allow("token transfer simulation succeeded — token is transferable")
        }
        Err(e) => {
            if e.contains("revert") || e.contains("execution reverted") {
                GuardrailResult::block(format!(
                    "HONEYPOT DETECTED: token {} reverted on transfer simulation. This token cannot be sold or transferred. — BLOCK",
                    token
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
}
