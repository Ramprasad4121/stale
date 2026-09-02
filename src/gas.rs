use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

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

    let current_gwei = (gas_price_wei / 1_000_000_000) as u64;

    if current_gwei > max_gas_price_gwei {
        GuardrailResult::block(format!(
            "network gas price {} gwei exceeds maximum allowed {} gwei — BLOCK",
            current_gwei, max_gas_price_gwei
        ))
    } else {
        GuardrailResult::allow(format!(
            "network gas price {} gwei is within safe limits",
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
