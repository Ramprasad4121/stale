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

/// EIP-1559 circuit breaker: BLOCK when `baseFeePerGas` exceeds
/// `max_base_fee_gwei` OR `maxPriorityFeePerGas` exceeds
/// `max_priority_fee_gwei` (or on RPC failure). Legacy `eth_gasPrice`
/// underestimates congestion on 1559 chains because it blends the burned
/// base fee with the tip; enforce both legs independently. Comparisons are
/// integer (`u128` wei); `f64` gwei values are display-only.
pub async fn check_gas_price_1559(
    client: &dyn EvmRpcClient,
    max_base_fee_gwei: u64,
    max_priority_fee_gwei: u64,
) -> GuardrailResult {
    if max_base_fee_gwei == 0 || max_priority_fee_gwei == 0 {
        return GuardrailResult::block(
            "maxBaseFeeGwei and maxPriorityFeeGwei must be > 0 — BLOCK (fail closed)",
        );
    }

    let base_fee_wei = match client.get_base_fee().await {
        Ok(f) => f,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to fetch baseFeePerGas — BLOCK (fail closed): {}",
                e
            ));
        }
    };
    let priority_fee_wei = match client.get_priority_fee().await {
        Ok(f) => f,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to fetch maxPriorityFeePerGas — BLOCK (fail closed): {}",
                e
            ));
        }
    };

    let max_base_wei = (max_base_fee_gwei as u128).saturating_mul(1_000_000_000);
    let max_priority_wei = (max_priority_fee_gwei as u128).saturating_mul(1_000_000_000);

    if base_fee_wei > max_base_wei {
        return GuardrailResult::block(format!(
            "baseFeePerGas {:.2} gwei exceeds maximum {} gwei — BLOCK",
            base_fee_wei as f64 / 1_000_000_000.0,
            max_base_fee_gwei
        ));
    }
    if priority_fee_wei > max_priority_wei {
        return GuardrailResult::block(format!(
            "maxPriorityFeePerGas {:.2} gwei exceeds maximum {} gwei — BLOCK",
            priority_fee_wei as f64 / 1_000_000_000.0,
            max_priority_fee_gwei
        ));
    }

    GuardrailResult::allow(format!(
        "baseFee {:.2} gwei + priority {:.2} gwei within safe limits",
        base_fee_wei as f64 / 1_000_000_000.0,
        priority_fee_wei as f64 / 1_000_000_000.0
    ))
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

    #[tokio::test]
    async fn test_1559_within_limits() {
        let mock = MockRpcClient {
            base_fee: Some(20_000_000_000),    // 20 gwei
            priority_fee: Some(1_000_000_000), // 1 gwei
            ..Default::default()
        };

        let res = check_gas_price_1559(&mock, 50, 5).await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_1559_base_fee_breach_blocks() {
        let mock = MockRpcClient {
            base_fee: Some(80_000_000_000),
            priority_fee: Some(1_000_000_000),
            ..Default::default()
        };

        let res = check_gas_price_1559(&mock, 50, 5).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("baseFeePerGas"));
    }

    #[tokio::test]
    async fn test_1559_priority_fee_breach_blocks() {
        let mock = MockRpcClient {
            base_fee: Some(20_000_000_000),
            priority_fee: Some(10_000_000_000), // 10 gwei tip spike
            ..Default::default()
        };

        let res = check_gas_price_1559(&mock, 50, 5).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("maxPriorityFeePerGas"));
    }

    #[tokio::test]
    async fn test_1559_missing_feed_blocks_fail_closed() {
        let mock = MockRpcClient::default();

        let res = check_gas_price_1559(&mock, 50, 5).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("fail closed"));
    }

    #[tokio::test]
    async fn test_1559_zero_policy_rejected() {
        let mock = MockRpcClient {
            base_fee: Some(1),
            priority_fee: Some(1),
            ..Default::default()
        };

        assert!(!check_gas_price_1559(&mock, 0, 5).await.allow_execute);
        assert!(!check_gas_price_1559(&mock, 50, 0).await.allow_execute);
    }
}
