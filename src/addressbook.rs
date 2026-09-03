//! Strict allowlist for contract targets.
//!
//! The simplest and strongest access control: unknown addresses BLOCK in
//! strict mode (the default you should use). Non-strict mode logs-but-allows
//! and is fail-open by configuration — only use it for discovery dry-runs.

use crate::types::GuardrailResult;
use std::collections::HashMap;

/// Lowercased-address → human label allowlist.
pub struct AddressBook {
    addresses: HashMap<String, String>, // lowercase address -> label
    strict: bool,
}

impl AddressBook {
    /// Build from `label → address` pairs. `Err` on any invalid address.
    /// `strict = true` is recommended (unknown → BLOCK). `strict = false`
    /// is fail-open by configuration — discovery dry-runs only; prefer
    /// [`AddressBook::new_strict`].
    pub fn new(allowlist: HashMap<String, String>, strict: bool) -> Result<Self, String> {
        let mut addresses = HashMap::new();
        for (label, addr) in allowlist {
            if !is_valid_eth_address(&addr) {
                return Err(format!("invalid address for \"{}\": {}", label, addr));
            }
            addresses.insert(addr.to_lowercase(), label);
        }
        Ok(Self { addresses, strict })
    }

    /// Strict constructor: unknown addresses BLOCK. Prefer this over
    /// `new(_, false)` everywhere except discovery dry-runs.
    pub fn new_strict(allowlist: HashMap<String, String>) -> Result<Self, String> {
        Self::new(allowlist, true)
    }

    /// Check `address`: invalid → BLOCK; known → ALLOW; unknown → BLOCK
    /// in strict mode, ALLOW (logged) otherwise.
    pub fn check(&self, address: &str) -> GuardrailResult {
        if !is_valid_eth_address(address) {
            return GuardrailResult::block(format!("invalid address {} — BLOCK", address));
        }

        let normalized = address.to_lowercase();
        if let Some(label) = self.addresses.get(&normalized) {
            GuardrailResult::allow(format!("address is allowlisted as \"{}\"", label))
        } else if self.strict {
            GuardrailResult::block(format!(
                "UNKNOWN ADDRESS: {} is NOT in the allowlist. The agent cannot interact with unknown contracts. — BLOCK",
                address
            ))
        } else {
            GuardrailResult::allow("address not in allowlist but strict mode is off")
        }
    }

    /// Membership test (case-insensitive).
    pub fn has(&self, address: &str) -> bool {
        self.addresses.contains_key(&address.to_lowercase())
    }

    /// Label for an allowlisted address, if present.
    pub fn label_of(&self, address: &str) -> Option<&String> {
        self.addresses.get(&address.to_lowercase())
    }

    /// Number of allowlisted entries.
    pub fn size(&self) -> usize {
        self.addresses.len()
    }
}

/// Strict `0x` + 40-hex validation. No EIP-55 checksum enforcement by
/// design (case is normalized on insert/lookup); checksum confusion is
/// out of scope — the allowlist comparison is case-insensitive.
pub fn is_valid_eth_address(addr: &str) -> bool {
    let clean = addr.trim();
    if !clean.starts_with("0x") && !clean.starts_with("0X") {
        return false;
    }
    let hex_part = &clean[2..];
    hex_part.len() == 40 && hex_part.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addressbook_allowlisted() {
        let mut map = HashMap::new();
        map.insert(
            "USDC".to_string(),
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        );
        let book = AddressBook::new(map, true).unwrap();

        let res = book.check("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
        assert!(res.allow_execute);
        assert!(res.reason.contains("allowlisted as \"USDC\""));
    }

    #[test]
    fn test_addressbook_strict_blocks_unknown() {
        let mut map = HashMap::new();
        map.insert(
            "USDC".to_string(),
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        );
        let book = AddressBook::new(map, true).unwrap();

        let res = book.check("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        assert!(!res.allow_execute);
        assert!(res.reason.contains("UNKNOWN ADDRESS"));
    }

    #[test]
    fn test_new_strict_blocks_unknown() {
        let mut map = HashMap::new();
        map.insert(
            "USDC".to_string(),
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48".to_string(),
        );
        let book = AddressBook::new_strict(map).unwrap();

        let res = book.check("0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045");
        assert!(!res.allow_execute);
    }
}
