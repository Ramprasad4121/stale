//! Solvency guard: native / ERC20 balance sufficiency.
//!
//! Check-then-act advisory (balance can move before broadcast). Any
//! RPC/decode failure → BLOCK.

use crate::abi::{decode_word_u128, encode_address_param};
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub const BALANCE_OF_SELECTOR: &str = "0x70a08231";

/// Require `agent` balance (`token` ERC20 or native ETH) `>= required_amount`,
/// plus a native-ETH `gas_reserve_wei` for execution costs. An agent that is
/// exactly solvent but cannot pay gas is still BLOCKed.
///
/// - Native path: `balance >= required_amount + gas_reserve_wei` (checked).
/// - Token path: token balance `>= required_amount` AND native balance
///   `>= gas_reserve_wei` (a second `eth_getBalance` read, fail closed).
/// - `gas_reserve_wei == 0` preserves the legacy exact-solvency semantics.
pub async fn check_balance(
    client: &dyn EvmRpcClient,
    agent: &str,
    token: Option<&str>,
    required_amount: u128,
    gas_reserve_wei: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(agent) {
        return GuardrailResult::block(format!("invalid agent address {} — BLOCK", agent));
    }
    if let Some(t) = token {
        if !is_valid_eth_address(t) {
            return GuardrailResult::block(format!("invalid token address {} — BLOCK", t));
        }
    }

    if required_amount == 0 && gas_reserve_wei == 0 {
        return GuardrailResult::block(
            "vacuous solvency check (required 0 + reserve 0): an empty wallet would ALLOW — BLOCK (fail closed)",
        );
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

    if let Some(t) = token {
        // Token path: the token balance covers the spend, but gas is paid
        // in native ETH — require the reserve natively as well.
        if gas_reserve_wei > 0 {
            let native = match client.get_balance(agent).await {
                Ok(b) => b,
                Err(e) => {
                    return GuardrailResult::block(format!(
                        "failed to fetch native gas reserve for agent {} — BLOCK (fail closed): {}",
                        agent, e
                    ));
                }
            };
            if native < gas_reserve_wei {
                return GuardrailResult::block(format!(
                    "no gas: agent {} native balance {} < gas reserve {} — BLOCK",
                    agent, native, gas_reserve_wei
                ));
            }
        }
        if balance < required_amount {
            return GuardrailResult::block(format!(
                "insolvent: agent {} has {} < required {} of {} — BLOCK",
                agent, balance, required_amount, t
            ));
        }
        return GuardrailResult::allow("agent strictly solvent (token + gas reserve)");
    }

    let total = match required_amount.checked_add(gas_reserve_wei) {
        Some(t) => t,
        None => {
            return GuardrailResult::block(
                "required + gas reserve overflows u128 — BLOCK (fail closed)",
            )
        }
    };
    if balance < total {
        let asset = token.unwrap_or("native ETH");
        GuardrailResult::block(format!(
            "insolvent: agent {} has {} < required {} + gas reserve {} of {} — BLOCK",
            agent, balance, required_amount, gas_reserve_wei, asset
        ))
    } else {
        GuardrailResult::allow("agent strictly solvent (amount + gas reserve)")
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
            100_000_000_000_000_000, // 0.1 ETH gas reserve
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
            0,
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
            0, // legacy exact-solvency semantics
        )
        .await;

        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_native_gas_reserve_breach_blocks() {
        let mock = MockRpcClient {
            balance: Some(10_000_000_000_000_000_000), // 10 ETH
            ..Default::default()
        };

        // 9.95 required + 0.1 reserve = 10.05 > 10 → BLOCK despite solvency.
        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            None,
            9_950_000_000_000_000_000,
            100_000_000_000_000_000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("gas reserve"));
    }

    #[tokio::test]
    async fn test_reserve_overflow_blocks_fail_closed() {
        let mock = MockRpcClient {
            balance: Some(u128::MAX),
            ..Default::default()
        };

        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            None,
            u128::MAX,
            1,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("overflows"));
    }

    #[tokio::test]
    async fn test_erc20_without_native_gas_blocks() {
        let hex = format!("0x{:0>64x}", 5_000_000_000u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(hex.clone()))),
            balance: Some(0), // token-rich, gas-broke
            ..Default::default()
        };

        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            Some("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"),
            1_000_000_000,
            100_000_000_000_000_000,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("no gas"));
    }

    #[tokio::test]
    async fn test_zero_zero_check_blocked_as_vacuous() {
        let mock = MockRpcClient {
            balance: Some(0),
            ..Default::default()
        };

        let res = check_balance(
            &mock,
            "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
            None,
            0,
            0,
        )
        .await;

        assert!(!res.allow_execute);
        assert!(res.reason.contains("vacuous"));
    }
}
