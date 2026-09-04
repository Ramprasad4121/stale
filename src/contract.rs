//! EOA / phishing guard: require deployed bytecode at the target.
//!
//! Blocks EOAs (empty code) and EIP-7702 delegated EOAs (`0xef0100…`),
//! which are revocable delegations — not immutable contracts.
//!
//! # What this does NOT prove
//! Has-bytecode is necessary but not sufficient: proxies, upgradeable
//! contracts, `delegatecall` targets, and honeypots all HAVE bytecode and
//! pass this guard. Compose with [`crate::addressbook::AddressBook`]
//! (allowlist known contracts) for unknown targets.

use crate::addressbook::is_valid_eth_address;
use crate::rpc::EvmRpcClient;
use crate::types::GuardrailResult;

/// Require `address` to host real contract bytecode (`eth_getCode`).
///
/// Besides `""`/`0x`, rejects `0x0`/`0x00…` (empty code with zero padding)
/// and odd-length / non-hex payloads. Fail closed on transport error.
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
    let body = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);
    // Empty, non-hex, odd-length, or all-zero code = EOA / no contract.
    let is_empty_code = body.is_empty()
        || body.len() % 2 != 0
        || !body.chars().all(|c| c.is_ascii_hexdigit())
        || body.chars().all(|c| c == '0');
    if is_empty_code {
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
    async fn test_zero_padded_empty_code_blocked() {
        for code in ["0x00", "0x0000", "0x000000"] {
            let mock = MockRpcClient {
                code: Some(code.to_string()),
                ..Default::default()
            };
            let res = check_is_contract(&mock, "0x95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5").await;
            assert!(!res.allow_execute, "code {} must BLOCK", code);
        }
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
