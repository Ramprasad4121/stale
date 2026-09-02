use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub async fn check_rpc_sync(
    client: &dyn EvmRpcClient,
    max_block_age_seconds: u64,
    now_seconds: u64,
) -> GuardrailResult {
    if max_block_age_seconds == 0 {
        return GuardrailResult::block("invalid maxBlockAgeSeconds — BLOCK");
    }

    let block_time = match client.get_block_timestamp().await {
        Ok(t) => t,
        Err(e) => {
            return GuardrailResult::block(format!("failed to fetch latest block — BLOCK: {}", e));
        }
    };

    if now_seconds < block_time {
        return GuardrailResult::block(format!(
            "future block timestamp: block {} is ahead of now {} — BLOCK",
            block_time, now_seconds
        ));
    }

    let age = now_seconds - block_time;

    if age > max_block_age_seconds {
        GuardrailResult::block(format!(
            "RPC STALL DANGER: latest block is {} seconds old (max {}). The RPC is out of sync. — BLOCK",
            age, max_block_age_seconds
        ))
    } else {
        GuardrailResult::allow(format!("RPC is synced (block age {}s)", age))
    }
}

pub async fn check_chain_id(client: &dyn EvmRpcClient, expected_chain_id: u64) -> GuardrailResult {
    if expected_chain_id == 0 {
        return GuardrailResult::block("invalid expectedChainId — BLOCK");
    }

    let current_chain_id = match client.get_chain_id().await {
        Ok(c) => c,
        Err(e) => {
            return GuardrailResult::block(format!("failed to fetch chain id — BLOCK: {}", e));
        }
    };

    if current_chain_id != expected_chain_id {
        GuardrailResult::block(format!(
            "CHAIN MISMATCH DANGER: expected chain {}, but RPC is on chain {}. — BLOCK",
            expected_chain_id, current_chain_id
        ))
    } else {
        GuardrailResult::allow("RPC chain ID matches expected")
    }
}

pub async fn check_nonce(
    client: &dyn EvmRpcClient,
    agent: &str,
    expected_nonce: u64,
) -> GuardrailResult {
    if !is_valid_eth_address(agent) {
        return GuardrailResult::block(format!("invalid agent address {} — BLOCK", agent));
    }

    let network_nonce = match client.get_transaction_count(agent).await {
        Ok(n) => n,
        Err(e) => {
            return GuardrailResult::block(format!("failed to fetch nonce — BLOCK: {}", e));
        }
    };

    if network_nonce > expected_nonce {
        GuardrailResult::block(format!(
            "STATE DESYNC: network nonce ({}) is higher than expected ({}). Previous transactions have confirmed. — BLOCK",
            network_nonce, expected_nonce
        ))
    } else {
        GuardrailResult::allow("nonce is in sync")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;

    #[tokio::test]
    async fn test_rpc_sync_fresh() {
        let mock = MockRpcClient {
            block_timestamp: Some(1000),
            ..Default::default()
        };

        let res = check_rpc_sync(&mock, 60, 1020).await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_rpc_sync_stalled() {
        let mock = MockRpcClient {
            block_timestamp: Some(1000),
            ..Default::default()
        };

        let res = check_rpc_sync(&mock, 60, 1100).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("RPC STALL DANGER"));
    }

    #[tokio::test]
    async fn test_chain_id_match() {
        let mock = MockRpcClient {
            chain_id: Some(1),
            ..Default::default()
        };

        let res = check_chain_id(&mock, 1).await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_chain_id_mismatch() {
        let mock = MockRpcClient {
            chain_id: Some(42161),
            ..Default::default()
        };

        let res = check_chain_id(&mock, 1).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("CHAIN MISMATCH DANGER"));
    }

    #[tokio::test]
    async fn test_nonce_sync() {
        let mock = MockRpcClient {
            nonce: Some(5),
            ..Default::default()
        };

        let res = check_nonce(&mock, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", 5).await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_nonce_desync() {
        let mock = MockRpcClient {
            nonce: Some(6),
            ..Default::default()
        };

        let res = check_nonce(&mock, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", 5).await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("STATE DESYNC"));
    }
}
