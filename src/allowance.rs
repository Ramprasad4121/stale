use crate::abi::{decode_word_u128, encode_address_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const ALLOWANCE_SELECTOR: &str = "0xdd62ed3e";

// u128::MAX is a safe proxy for max approval check, or 2^127
pub const MAX_U128: u128 = u128::MAX;
pub const DANGEROUSLY_LARGE_U128: u128 = 1u128 << 126;

pub fn check_approval(token: &str, spender: &str, amount: u128) -> GuardrailResult {
    if !is_valid_eth_address(token) {
        return GuardrailResult::block(format!("invalid token address {} — BLOCK", token));
    }
    if !is_valid_eth_address(spender) {
        return GuardrailResult::block(format!("invalid spender address {} — BLOCK", spender));
    }

    if amount == MAX_U128 {
        return GuardrailResult::block(format!(
            "infinite approval to spender {} is strictly forbidden — BLOCK",
            spender
        ));
    }

    if amount > DANGEROUSLY_LARGE_U128 {
        return GuardrailResult::block(format!(
            "dangerously large approval to spender {} — BLOCK",
            spender
        ));
    }

    GuardrailResult::allow(format!("approval to {} is a safe exact amount", spender))
}

pub async fn check_allowance(
    client: &dyn EvmRpcClient,
    token: &str,
    owner: &str,
    spender: &str,
    required_amount: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(token) {
        return GuardrailResult::block(format!("invalid token address {} — BLOCK", token));
    }
    if !is_valid_eth_address(owner) {
        return GuardrailResult::block(format!("invalid owner address {} — BLOCK", owner));
    }
    if !is_valid_eth_address(spender) {
        return GuardrailResult::block(format!("invalid spender address {} — BLOCK", spender));
    }

    let (owner_enc, spender_enc) =
        match (encode_address_param(owner), encode_address_param(spender)) {
            (Ok(o), Ok(s)) => (o, s),
            _ => {
                return GuardrailResult::block(
                    "invalid owner/spender address encoding — BLOCK (fail closed)".to_string(),
                )
            }
        };
    let calldata = format!("{}{}{}", ALLOWANCE_SELECTOR, owner_enc, spender_enc);

    let current_allowance = match client.call(token, &calldata).await {
        Ok(hex_data) => match decode_word_u128(&hex_data, 0) {
            Ok(amt) => amt,
            Err(e) => {
                return GuardrailResult::block(format!(
                    "failed to decode allowance response — BLOCK (fail closed): {}",
                    e
                ));
            }
        },
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to read allowance — BLOCK (fail closed): {}",
                e
            ));
        }
    };

    if current_allowance < required_amount {
        GuardrailResult::block(format!(
            "insufficient allowance: owner {} has only approved {} < required {} for spender {} — BLOCK",
            owner, current_allowance, required_amount, spender
        ))
    } else {
        GuardrailResult::allow("allowance strictly meets required amount")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[test]
    fn test_exact_approval_allowed() {
        let res = check_approval(
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
            1_000_000_000,
        );
        assert!(res.allow_execute);
    }

    #[test]
    fn test_infinite_approval_blocked() {
        let res = check_approval(
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
            u128::MAX,
        );
        assert!(!res.allow_execute);
        assert!(res.reason.contains("infinite approval"));
    }

    #[tokio::test]
    async fn test_allowance_sufficient() {
        let hex = format!("0x{:0>64x}", 5000u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            ..Default::default()
        };

        let res = check_allowance(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
            1000,
        )
        .await;

        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_allowance_insufficient() {
        let hex = format!("0x{:0>64x}", 500u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            ..Default::default()
        };

        let res = check_allowance(
            &mock,
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
            1000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("insufficient allowance"));
    }
}
