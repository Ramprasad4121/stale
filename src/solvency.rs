use crate::abi::{decode_word_u128, encode_address_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const BALANCE_OF_SELECTOR: &str = "0x70a08231";

pub async fn check_balance(
    client: &dyn EvmRpcClient,
    agent: &str,
    token: Option<&str>,
    required_amount: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(agent) {
        return GuardrailResult::block(format!("invalid agent address {} — BLOCK", agent));
    }
    if let Some(t) = token {
        if !is_valid_eth_address(t) {
            return GuardrailResult::block(format!("invalid token address {} — BLOCK", t));
        }
    }

    let balance = if let Some(token_addr) = token {
        let encoded = match encode_address_param(agent) {
            Ok(e) => e,
            Err(e) => return GuardrailResult::block(format!("{} — BLOCK (fail closed)", e)),
        };
        let calldata = format!("{}{}", BALANCE_OF_SELECTOR, encoded);
        match client.call(token_addr, &calldata).await {
            Ok(hex_data) => match decode_word_u128(&hex_data, 0) {
                Ok(b) => b,
                Err(e) => {
                    return GuardrailResult::block(format!(
                        "failed to decode token balance response — BLOCK (fail closed): {}",
                        e
                    ));
                }
            },
            Err(e) => {
                return GuardrailResult::block(format!(
                    "failed to fetch token balance for agent {} — BLOCK (fail closed): {}",
                    agent, e
                ));
            }
        }
    } else {
        match client.get_balance(agent).await {
            Ok(b) => b,
            Err(e) => {
                return GuardrailResult::block(format!(
                    "failed to fetch native balance for agent {} — BLOCK (fail closed): {}",
                    agent, e
                ));
            }
        }
    };

    if balance < required_amount {
        let asset = token.unwrap_or("native ETH");
        GuardrailResult::block(format!(
            "insolvent: agent {} has {} < required {} of {} — BLOCK",
            agent, balance, required_amount, asset
        ))
    } else {
        GuardrailResult::allow("agent strictly solvent")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_native_balance_solvent() {
        let mock = MockRpcClient {
            balance: Some(10_000_000_000_000_000_000), // 10 ETH
            ..Default::default()
        };

        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            None,
            1_000_000_000_000_000_000,
        )
        .await;

        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_native_balance_insolvent() {
        let mock = MockRpcClient {
            balance: Some(500_000_000_000_000_000), // 0.5 ETH
            ..Default::default()
        };

        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            None,
            1_000_000_000_000_000_000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("insolvent"));
    }

    #[tokio::test]
    async fn test_erc20_balance_solvent() {
        let hex = format!("0x{:0>64x}", 5_000_000_000u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            ..Default::default()
        };

        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            1_000_000_000,
        )
        .await;

        assert!(res.allow_execute);
    }
}
