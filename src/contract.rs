use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

pub async fn check_is_contract(client: &dyn EvmRpcClient, address: &str) -> GuardrailResult {
    if !is_valid_eth_address(address) {
        return GuardrailResult::block(format!("invalid address {} — BLOCK", address));
    }

    let code = match client.get_code(address).await {
        Ok(c) => c,
        Err(e) => {
            return GuardrailResult::block(format!(
                "failed to fetch bytecode for {} — BLOCK (fail closed): {}",
                address, e
            ));
        }
    };

    let trimmed = code.trim();
    if trimmed.is_empty() || trimmed == "0x" || trimmed == "0X" {
        return GuardrailResult::block(format!(
            "PHISHING DANGER: address {} is an EOA (Externally Owned Account) with no bytecode. Do not approve or route funds to EOAs. — BLOCK",
            address
        ));
    }

    // EIP-7702 detection: 0xef0100 + 20-byte address indicates a delegated EOA, not a contract
    if trimmed.starts_with("0xef0100") || trimmed.starts_with("0xEF0100") {
        return GuardrailResult::block(format!(
            "PHISHING DANGER: address {} is an EIP-7702 delegated EOA, not an immutable smart contract. — BLOCK",
            address
        ));
    }

    GuardrailResult::allow(format!("address {} is a deployed smart contract", address))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::MockRpcClient;

    #[tokio::test]
    async fn test_smart_contract_allowed() {
        let mock = MockRpcClient {
            code: Some("0x608060405234801561001057600080fd5b50".to_string()),
            ..Default::default()
        };

        let res = check_is_contract(&mock, "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48").await;
        assert!(res.allow_execute);
    }

    #[tokio::test]
    async fn test_eoa_blocked() {
        let mock = MockRpcClient {
            code: Some("0x".to_string()),
            ..Default::default()
        };

        let res = check_is_contract(&mock, "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("PHISHING DANGER"));
    }

    #[tokio::test]
    async fn test_eip7702_delegated_eoa_blocked() {
        let mock = MockRpcClient {
            code: Some("0xef01005a7fc11397e9a8ad41bf10bf13f22b0a63f96f6d".to_string()),
            ..Default::default()
        };

        let res = check_is_contract(&mock, "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045").await;
        assert!(!res.allow_execute);
        assert!(res.reason.contains("EIP-7702"));
    }
}
