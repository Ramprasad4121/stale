use crate::abi::decode_bool;
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const PAUSED_SELECTOR: &str = "0x5c975abb";

pub async fn check_paused(client: &dyn EvmRpcClient, contract: &str) -> GuardrailResult {
    if !is_valid_eth_address(contract) {
        return GuardrailResult::block(format!("invalid contract address {} — BLOCK", contract));
    }

    let is_paused = match client.call(contract, PAUSED_SELECTOR).await {
        Ok(hex_data) => match decode_bool(&hex_data) {
            Ok(b) => b,
            Err(_) => {
                // Not implementing paused() returns ALLOW
                return GuardrailResult::allow(format!(
                    "contract {} does not implement paused() — safely ALLOW",
                    contract
                ));
            }
        },
        Err(_) => {
            // If the call reverts, contract does not implement paused()
            return GuardrailResult::allow(format!(
                "contract {} does not implement paused() or call reverted — safely ALLOW",
                contract
            ));
        }
    };

    if is_paused {
        GuardrailResult::block(format!("contract {} is currently PAUSED — BLOCK", contract))
    } else {
        GuardrailResult::allow(format!("contract {} is active (not paused)", contract))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_contract_paused() {
        let paused_hex = format!("0x{:0>64x}", 1u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(paused_hex.clone()))),
            ..Default::default()
        };

        let res = check_paused(&mock, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("PAUSED"));
    }

    #[tokio::test]
    async fn test_contract_not_paused() {
        let paused_hex = format!("0x{:0>64x}", 0u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(paused_hex.clone()))),
            ..Default::default()
        };

        let res = check_paused(&mock, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").await;
        assert!(res.allow_execute);
        assert!(res.reason.contains("active (not paused)"));
    }

    #[tokio::test]
    async fn test_contract_not_pausable_defaults_to_allow() {
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Err("execution reverted".to_string()))),
            ..Default::default()
        };

        let res = check_paused(&mock, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").await;
        assert!(res.allow_execute);
        assert!(res.reason.contains("does not implement paused()"));
    }
}
