//! Gas circuit breaker: BLOCK when `eth_gasPrice` exceeds policy.
//!
//! Comparison is integer (`u128` wei); the `f64` gwei value in the reason
//! string is display-only.

use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

/// BLOCK if network gas price exceeds `max_gas_price_gwei` (or on RPC
/// failure). `max_gas_price_gwei == 0` is rejected as misconfiguration.
pub async fn check_gas_price(
    client: &dyn EvmRpcClient,
    max_gas_price_gwei: u64,
) -> GuardrailResult {
    if max_gas_price_gwei == 0 {
        return GuardrailResult::block("maxGasPriceGwei must be > 0 — BLOCK (fail closed)");
    }

    let gas_price_wei = match client.get_gas_price().await {
        Ok(p) => p,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to fetch network gas price — BLOCK (fail closed): {}",
                e
            ));
        }
    };

    let max_gas_price_wei = (max_gas_price_gwei as u128).saturating_mul(1_000_000_000);
    let current_gwei = (gas_price_wei as f64) / 1_000_000_000.0;

    if gas_price_wei > max_gas_price_wei {
        GuardrailResult::block(format!(
            "network gas price {:.2} gwei exceeds maximum allowed {} gwei — BLOCK",
            current_gwei, max_gas_price_gwei
        ))
    } else {
        GuardrailResult::allow(format!(
            "network gas price {:.2} gwei is within safe limits",
            current_gwei
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;

    #[tokio::test]
    async fn test_gas_price_within_limits() {
        let mock = MockRpcClient {
            gas_price: Some(30_000_000_000), // 30 gwei
            ..Default::default()
        };

        let res = check_gas_price(&mock, 50).await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_gas_price_borderline_fraction_blocked() {
        let mock = MockRpcClient {
            gas_price: Some(50_500_000_000), // 50.5 gwei
            ..Default::default()
        };

        let res = check_gas_price(&mock, 50).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("exceeds maximum allowed"));
    }

    #[tokio::test]
    async fn test_gas_price_exceeds_limits() {
        let mock = MockRpcClient {
            gas_price: Some(80_000_000_000), // 80 gwei
            ..Default::default()
        };

        let res = check_gas_price(&mock, 50).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("exceeds maximum allowed"));
    }

    #[tokio::test]
    async fn test_gas_price_rpc_failure_blocks() {
        let mock = MockRpcClient::default(); // unconfigured gas price returns error

        let res = check_gas_price(&mock, 50).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("failed to fetch network gas price"));
    }
}
