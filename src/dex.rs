use crate::abi::decode_word_u128;
use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;
use serde_json::json;

pub const V2_GET_RESERVES_SELECTOR: &str = "0x0902f1ac";
pub const V3_LIQUIDITY_SELECTOR: &str = "0x1a686502";

pub async fn check_pool_v2(
    client: &dyn EvmRpcClient,
    pool: &str,
    min_reserve0: u128,
    min_reserve1: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(pool) {
        return GuardrailResult::block(format!("invalid pool address {} — BLOCK", pool));
    }

    let hex_data = match client.call(pool, V2_GET_RESERVES_SELECTOR).await {
        Ok(h) => h,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to read reserves from V2 pool {} — BLOCK (fail closed): {}",
                pool, e
            ));
        }
    };

    let reserve0 = match decode_word_u128(&hex_data, 0) {
        Ok(r) => r,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to decode reserve0 from V2 pool {}: {}",
                pool, e
            ));
        }
    };

    let reserve1 = match decode_word_u128(&hex_data, 1) {
        Ok(r) => r,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to decode reserve1 from V2 pool {}: {}",
                pool, e
            ));
        }
    };

    if reserve0 < min_reserve0 {
        return GuardrailResult::block(format!(
            "reserve0 {} < required {} on V2 pool {} — BLOCK",
            reserve0, min_reserve0, pool
        ))
        .with_metadata(json!({
            "reserve0": reserve0.to_string(),
            "reserve1": reserve1.to_string(),
            "pool": pool
        }));
    }

    if reserve1 < min_reserve1 {
        return GuardrailResult::block(format!(
            "reserve1 {} < required {} on V2 pool {} — BLOCK",
            reserve1, min_reserve1, pool
        ))
        .with_metadata(json!({
            "reserve0": reserve0.to_string(),
            "reserve1": reserve1.to_string(),
            "pool": pool
        }));
    }

    GuardrailResult::allow(format!(
        "V2 pool {} reserves meet minimum requirements",
        pool
    ))
    .with_metadata(json!({
        "reserve0": reserve0.to_string(),
        "reserve1": reserve1.to_string(),
        "pool": pool
    }))
}

pub async fn check_pool_v3(
    client: &dyn EvmRpcClient,
    pool: &str,
    min_liquidity: u128,
) -> GuardrailResult {
    if !is_valid_eth_address(pool) {
        return GuardrailResult::block(format!("invalid pool address {} — BLOCK", pool));
    }

    let hex_data = match client.call(pool, V3_LIQUIDITY_SELECTOR).await {
        Ok(h) => h,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to read liquidity from V3 pool {} — BLOCK (fail closed): {}",
                pool, e
            ));
        }
    };

    let active_liquidity = match decode_word_u128(&hex_data, 0) {
        Ok(l) => l,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to decode liquidity from V3 pool {}: {}",
                pool, e
            ));
        }
    };

    if active_liquidity < min_liquidity {
        return GuardrailResult::block(format!(
            "active liquidity {} < required {} on V3 pool {} — BLOCK",
            active_liquidity, min_liquidity, pool
        ))
        .with_metadata(json!({
            "liquidity": active_liquidity.to_string(),
            "pool": pool
        }));
    }

    GuardrailResult::allow(format!(
        "V3 pool {} liquidity {} meets minimum requirements",
        pool, active_liquidity
    ))
    .with_metadata(json!({
        "liquidity": active_liquidity.to_string(),
        "pool": pool
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pool_v2_sufficient() {
        let reserves_hex = format!("0x{:0>64x}{:0>64x}{:0>64x}", 1000u64, 2000u64, 1u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(reserves_hex.clone()))),
            ..Default::default()
        };

        let res = check_pool_v2(
            &mock,
            "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
            500,
            1000,
        )
        .await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_pool_v2_insufficient() {
        let reserves_hex = format!("0x{:0>64x}{:0>64x}{:0>64x}", 300u64, 2000u64, 1u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(reserves_hex.clone()))),
            ..Default::default()
        };

        let res = check_pool_v2(
            &mock,
            "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
            500,
            1000,
        )
        .await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("reserve0 300 < required 500"));
    }

    #[tokio::test]
    async fn test_pool_v3_sufficient() {
        let liquidity_hex = format!("0x{:0>64x}", 50_000_000u64);
        let mock = MockRpcClient {
            call_handler: Some(Arc::new(move |_, _| Ok(liquidity_hex.clone()))),
            ..Default::default()
        };

        let res = check_pool_v3(
            &mock,
            "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
            10_000_000,
        )
        .await;
        assert!(res.allow_execute);
    }
}
