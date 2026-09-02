use crate::types::GuardrailResult;
use url::Url;

pub const MEV_PROTECTED_RPCS: &[&str] = &[
    "https://rpc.flashbots.net",
    "https://rpc.mevblocker.io",
    "https://eth.rpc.blxrbdn.com",
    "https://rpc.beaverbuild.org",
    "https://rpc.titanbuilder.xyz",
    "https://api.edennetwork.io/v1/rpc",
];

pub fn check_mev_rpc(rpc: &str) -> GuardrailResult {
    let trimmed = rpc.trim();
    if trimmed.is_empty() {
        return GuardrailResult::block("missing rpc — BLOCK");
    }

    let parsed_url = match Url::parse(trimmed) {
        Ok(u) => u,
        Err(_) => return GuardrailResult::block("invalid rpc url format — BLOCK"),
    };

    let target_host = match parsed_url.host_str() {
        Some(h) => h,
        None => return GuardrailResult::block("rpc url missing host — BLOCK"),
    };

    let is_protected = MEV_PROTECTED_RPCS.iter().any(|p| {
        if let Ok(p_url) = Url::parse(p) {
            if let Some(p_host) = p_url.host_str() {
                return target_host == p_host;
            }
        }
        false
    });

    if !is_protected {
        GuardrailResult::block(format!(
            "PUBLIC MEMPOOL DANGER: RPC {} is not a recognized MEV-protected endpoint. Transactions will be sandwiched. — BLOCK",
            target_host
        ))
    } else {
        GuardrailResult::allow(format!("RPC {} is MEV-protected", target_host))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flashbots_protected() {
        let res = check_mev_rpc("https://rpc.flashbots.net/fast");
        assert!(res.allow_execute);
    }

    #[test]
    fn test_mevblocker_protected() {
        let res = check_mev_rpc("https://rpc.mevblocker.io");
        assert!(res.allow_execute);
    }

    #[test]
    fn test_public_rpc_blocked() {
        let res = check_mev_rpc("https://eth-mainnet.g.alchemy.com/v2/demo");
        assert!(!res.allow_execute);
        assert!(res.reason.contains("PUBLIC MEMPOOL DANGER"));
    }
}
