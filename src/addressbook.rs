use crate::types::GuardrailResult;
use std::collections::HashMap;

pub struct AddressBook {
    addresses: HashMap<String, String>, // lowercase address -> label
    strict: bool,
}

impl AddressBook {
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

    pub fn has(&self, address: &str) -> bool {
        self.addresses.contains_key(&address.to_lowercase())
    }

    pub fn label_of(&self, address: &str) -> Option<&String> {
        self.addresses.get(&address.to_lowercase())
    }

    pub fn size(&self) -> usize {
        self.addresses.len()
    }
}

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
}
