//! Pre-flight execution simulation (`eth_call`).
//!
//! Revert → BLOCK. Check-then-act advisory: state can shift between
//! simulation and broadcast (frontrun, slot churn). Bind outcomes with
//! deadlines, slippage limits, and private mempools.

use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

#[derive(Debug, Clone)]
/// `account`: simulation sender. `to`: target. `data`: calldata (`0x` if none).
pub struct SimulateTxInput<'a> {
    pub account: &'a str,
    pub to: &'a str,
    pub data: Option<&'a str>,
}

/// Simulate via `eth_call(from, to, data)`. Any revert/transport error → BLOCK.
pub async fn simulate_tx(client: &dyn EvmRpcClient, input: SimulateTxInput<'_>) -> GuardrailResult {
    if !is_valid_eth_address(input.account) {
        return GuardrailResult::block(format!(
            "invalid account address {} — BLOCK",
            input.account
        ));
    }
    if !is_valid_eth_address(input.to) {
        return GuardrailResult::block(format!("invalid to address {} — BLOCK", input.to));
    }

    let calldata = input.data.unwrap_or("0x");

    match client.call_from(input.account, input.to, calldata).await {
        Ok(_) => GuardrailResult::allow("transaction simulation succeeded"),
        Err(e) => GuardrailResult::block(format!(
            "Simulation reverted! The transaction will fail on-chain. Do NOT execute. Revert reason: {}",
            e
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_simulation_succeeds() {
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok("0x".to_string()))),
            ..Default::default()
        };

        let res = simulate_tx(
            &mock,
            SimulateTxInput {
                account: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
                to: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                data: None,
            },
        )
        .await;

        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_simulation_reverts_blocks() {
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| {
                Err("reverted with UniswapV2: K".to_string())
            })),
            ..Default::default()
        };

        let res = simulate_tx(
            &mock,
            SimulateTxInput {
                account: "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
                to: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
                data: None,
            },
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("Simulation reverted"));
    }
}
