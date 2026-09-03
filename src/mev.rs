//! MEV-protection guard: require a recognized private-builder RPC.
//!
//! Public mempools expose agent transactions to frontrunning/sandwiching.
//! This guard allowlists known MEV-protected endpoints by **host** (path
//!-insensitive) and requires **`https`** except for loopback dev endpoints.

use crate::types::GuardrailResult;
use url::Url;

/// Recognized MEV-protected endpoints (matched by host).
pub const MEV_PROTECTED_RPCS: &[&str] = &[
    "https://rpc.flashbots.net",
    "https://rpc.mevblocker.io",
    "https://eth.rpc.blxrbdn.com",
    "https://rpc.beaverbuild.org",
    "https://rpc.titanbuilder.xyz",
    "https://api.edennetwork.io/v1/rpc",
];

/// Require `rpc` to be a recognized MEV-protected endpoint over HTTPS.
///
/// `http://` is accepted only for loopback (`localhost`, `127.0.0.0/8`,
/// `::1`) to keep local dev usable; every other `http://` URL BLOCKs even
/// if the host matches (plaintext builder traffic is MITM-able).
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

    let is_loopback = target_host == "localhost"
        || target_host == "::1"
        || target_host == "[::1]"
        || is_ipv4_loopback(target_host);
    if parsed_url.scheme() != "https" && !is_loopback {
        return GuardrailResult::block(format!(
            "INSECURE RPC: {} uses plaintext http — MEV-protected endpoints must use https. — BLOCK",
            target_host
        ));
    }

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

/// True for `127.0.0.0/8`.
fn is_ipv4_loopback(host: &str) -> bool {
    let mut parts = host.split('.');
    match (
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
        parts.next(),
    ) {
        (Some("127"), Some(a), Some(b), Some(c), None) => {
            [a, b, c].iter().all(|p| p.parse::<u8>().is_ok())
        }
        _ => false,
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

    #[test]
    fn test_plaintext_builder_blocked() {
        // Known host but plaintext http → MITM risk, must BLOCK.
        let res = check_mev_rpc("http://rpc.flashbots.net");
        assert!(!res.allow_execute);
        assert!(res.reason.contains("INSECURE RPC"));
    }

    #[test]
    fn test_localhost_subdomain_spoof_blocked() {
        // Prefix-spoof: host is localhost.evil.com, not loopback.
        let res = check_mev_rpc("http://localhost.evil.com");
        assert!(!res.allow_execute);
    }

    #[test]
    fn test_loopback_127_spoof_blocked() {
        let res = check_mev_rpc("http://127.evil.com");
        assert!(!res.allow_execute);
    }
}
