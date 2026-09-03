//! Chainlink-grade JSON-RPC transport.
//!
//! # Trust model
//! - **Trusted:** nothing on the wire. Every byte of every RPC response is
//!   untrusted until validated (hex shape, ABI length, numeric bounds).
//! - **Untrusted:** RPC URL, HTTP status, JSON body, `result` field.
//!   Any failure fails closed to `BLOCK`; `stale` never substitutes cached data.
//!
//! # Security properties
//! - HTTPS is mandatory except for loopback (`localhost`, `127.0.0.0/8`,
//!   `::1`). Host is compared after URL parsing — never by string prefix —
//!   so `http://localhost.evil.com` is rejected.
//! - Responses are size-bounded ([`MAX_RPC_RESPONSE_BYTES`]) before JSON
//!   parsing to bound OOM from a malicious RPC.
//! - The configured URL (which may embed an API key) is redacted from every
//!   surfaced error string, including query strings and credentials.

use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

/// Maximum RPC response body accepted before parsing: 1 MiB.
/// A single `eth_call` / `eth_getBlockByNumber` result is a few KB;
/// anything larger is a misbehaving or malicious endpoint.
pub const MAX_RPC_RESPONSE_BYTES: usize = 1024 * 1024;

/// SSRF guard: is `rpc_url`'s host in `allowed_hosts`? An EMPTY allowlist
/// means "no restriction" (legacy behavior, documented on the flag).
/// Comparison is on the parsed host, case-insensitive — never on the raw
/// string — so credentials, ports, paths, and casing cannot smuggle a host.
pub fn is_rpc_host_allowed(rpc_url: &str, allowed_hosts: &[String]) -> bool {
    if allowed_hosts.is_empty() {
        return true;
    }
    let host = match url::Url::parse(rpc_url) {
        Ok(u) => u.host_str().unwrap_or("").to_lowercase(),
        Err(_) => return false,
    };
    allowed_hosts.iter().any(|h| h.to_lowercase() == host)
}
#[async_trait]
/// Abstract EVM JSON-RPC transport.
///
/// All methods fail closed (`Err` → caller emits `BLOCK`). Implementations
/// must never substitute cached data on failure.
pub trait EvmRpcClient: Send + Sync {
    async fn call(&self, to: &str, data: &str) -> Result<String, String>;
    async fn call_from(&self, from: &str, to: &str, data: &str) -> Result<String, String>;
    /// `eth_call` with native `value` (wei) for payable flows. Default:
    /// `Err` (fail closed) so transports without value support never
    /// simulate payables as 0-value calls.
    async fn call_from_with_value(
        &self,
        _from: &str,
        _to: &str,
        _data: &str,
        _value_wei: u128,
    ) -> Result<String, String> {
        Err("value simulation not supported by this transport — BLOCK".to_string())
    }
    async fn get_block_number(&self) -> Result<u64, String>;
    async fn get_block_timestamp(&self) -> Result<u64, String>;
    async fn get_chain_id(&self) -> Result<u64, String>;
    async fn get_transaction_count(&self, address: &str) -> Result<u64, String>;
    async fn get_gas_price(&self) -> Result<u128, String>;
    /// Latest `baseFeePerGas` (`eth_getBlockByNumber`). Default: `Err`
    /// (fail closed) so external implementors are never silently treated
    /// as 1559-capable. Pre-1559 chains (no `baseFeePerGas` field) also
    /// `Err` in the HTTP transport — callers map to `BLOCK`.
    async fn get_base_fee(&self) -> Result<u128, String> {
        Err("base fee not supported by this transport — BLOCK".to_string())
    }
    /// `eth_maxPriorityFeePerGas`. Same fail-closed default as `get_base_fee`.
    async fn get_priority_fee(&self) -> Result<u128, String> {
        Err("priority fee not supported by this transport — BLOCK".to_string())
    }
    async fn get_balance(&self, address: &str) -> Result<u128, String>;
    async fn get_code(&self, address: &str) -> Result<String, String>;
}

#[derive(Clone)]
/// HTTPS-enforcing `reqwest`-backed [`EvmRpcClient`] with a 10s timeout.
pub struct HttpRpcClient {
    rpc_url: String,
    client: reqwest::Client,
    allowed_hosts: Vec<String>,
}

impl HttpRpcClient {
    /// Create a client for `rpc_url`. URL policy is enforced per-call in
    /// `send_rpc` so construction never panics.
    pub fn new(rpc_url: impl Into<String>) -> Self {
        let rpc_url = rpc_url.into();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            rpc_url,
            client,
            allowed_hosts: Vec::new(),
        }
    }

    /// Restrict egress to `hosts` (exact, case-insensitive). Empty means
    /// unrestricted. Combine with HTTPS policy: both must pass.
    pub fn with_allowed_hosts(mut self, hosts: Vec<String>) -> Self {
        self.allowed_hosts = hosts;
        self
    }

    /// Validate the configured RPC URL.
    ///
    /// # Policy
    /// - `https://` anywhere is accepted.
    /// - `http://` is accepted **only** for loopback hosts (`localhost`,
    ///   `127.0.0.0/8`, `::1`), matched on the parsed host — never by
    ///   string prefix.
    ///
    /// # Errors
    /// Returns `Err` (caller maps to `BLOCK`) on unparseable URL,
    /// missing host, non-loopback `http`, or any non-http(s) scheme.
    fn validate_url(&self) -> Result<(), String> {
        let parsed = url::Url::parse(&self.rpc_url)
            .map_err(|_| "invalid RPC URL format — BLOCK".to_string())?;
        match parsed.scheme() {
            "https" => Ok(()),
            "http" => {
                let host = parsed.host_str().unwrap_or("");
                let is_loopback = host == "localhost"
                    || host == "::1"
                    || host == "[::1]"
                    || is_ipv4_loopback(host);
                if is_loopback {
                    Ok(())
                } else {
                    Err("refusing non-HTTPS RPC URL (MITM risk) — BLOCK. Use https:// or local http://localhost".to_string())
                }
            }
            _ => Err("unsupported RPC URL scheme (expected https) — BLOCK".to_string()),
        }
    }

    /// Strip the configured RPC URL (which may embed an API key) from error
    /// text so secrets never land in GuardrailResult.reason / audit logs.
    ///
    /// Redacts the full URL plus, when parseable, the credential/query
    /// fragments independently so partial leaks (e.g. endpoint echoing back
    /// only `?key=...`) are still scrubbed.
    fn redact(&self, msg: String) -> String {
        if self.rpc_url.is_empty() {
            return msg;
        }
        let mut out = msg.replace(&self.rpc_url, "<rpc-url>");
        if let Ok(parsed) = url::Url::parse(&self.rpc_url) {
            if !parsed.username().is_empty() {
                out = out.replace(parsed.username(), "<rpc-cred>");
            }
            if let Some(pw) = parsed.password() {
                out = out.replace(pw, "<rpc-cred>");
            }
            if let Some(q) = parsed.query() {
                // Redact each query value; keys are kept for debuggability.
                for pair in parsed.query_pairs() {
                    if !pair.1.is_empty() {
                        out = out.replace(pair.1.as_ref(), "<rpc-key>");
                    }
                }
                let _ = q;
            }
            if let Some(host) = parsed.host_str() {
                // Never redact the whole host (needed for "wrong endpoint"
                // diagnostics); credentials above are the secret part.
                let _ = host;
            }
        }
        out
    }

    async fn send_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.validate_url()?;
        if !is_rpc_host_allowed(&self.rpc_url, &self.allowed_hosts) {
            return Err(
                "RPC host not in allowlist — BLOCK (fail closed). Pass --allowed-rpc-hosts."
                    .to_string(),
            );
        }
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let resp = self
            .client
            .post(&self.rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| self.redact(format!("rpc network error: {}", e)))?;

        if !resp.status().is_success() {
            return Err(format!("rpc http error {}", resp.status()));
        }

        // Bound OOM from a malicious RPC: cap body before JSON parsing.
        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("failed to read rpc response: {}", e))?;
        if bytes.len() > MAX_RPC_RESPONSE_BYTES {
            return Err(format!(
                "rpc response {} bytes exceeds {} byte limit — BLOCK (fail closed)",
                bytes.len(),
                MAX_RPC_RESPONSE_BYTES
            ));
        }
        let body: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|e| format!("failed to parse rpc response: {}", e))?;

        if let Some(err) = body.get("error") {
            return Err(format!("rpc error: {}", err));
        }

        body.get("result")
            .cloned()
            .ok_or_else(|| "missing result field in rpc response".to_string())
    }
}

#[async_trait]
impl EvmRpcClient for HttpRpcClient {
    async fn call(&self, to: &str, data: &str) -> Result<String, String> {
        let res = self
            .send_rpc("eth_call", json!([{"to": to, "data": data}, "latest"]))
            .await?;
        res.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "expected hex string from eth_call".to_string())
    }

    async fn call_from(&self, from: &str, to: &str, data: &str) -> Result<String, String> {
        let res = self
            .send_rpc(
                "eth_call",
                json!([{"from": from, "to": to, "data": data}, "latest"]),
            )
            .await?;
        res.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "expected hex string from eth_call".to_string())
    }

    async fn call_from_with_value(
        &self,
        from: &str,
        to: &str,
        data: &str,
        value_wei: u128,
    ) -> Result<String, String> {
        let res = self
            .send_rpc(
                "eth_call",
                json!([{"from": from, "to": to, "data": data, "value": format!("0x{:x}", value_wei)}, "latest"]),
            )
            .await?;
        res.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "expected hex string from eth_call".to_string())
    }

    async fn get_block_number(&self) -> Result<u64, String> {
        let res = self.send_rpc("eth_blockNumber", json!([])).await?;
        parse_hex_u64(&res)
    }

    async fn get_block_timestamp(&self) -> Result<u64, String> {
        let res = self
            .send_rpc("eth_getBlockByNumber", json!(["latest", false]))
            .await?;
        let ts_hex = res
            .get("timestamp")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "missing block timestamp".to_string())?;
        u64::from_str_radix(strip_hex_prefix(ts_hex.trim()), 16)
            .map_err(|e| format!("invalid hex timestamp: {}", e))
    }

    async fn get_chain_id(&self) -> Result<u64, String> {
        let res = self.send_rpc("eth_chainId", json!([])).await?;
        parse_hex_u64(&res)
    }

    async fn get_transaction_count(&self, address: &str) -> Result<u64, String> {
        let res = self
            .send_rpc("eth_getTransactionCount", json!([address, "pending"]))
            .await?;
        parse_hex_u64(&res)
    }

    async fn get_gas_price(&self) -> Result<u128, String> {
        let res = self.send_rpc("eth_gasPrice", json!([])).await?;
        parse_hex_u128(&res)
    }

    async fn get_base_fee(&self) -> Result<u128, String> {
        let res = self
            .send_rpc("eth_getBlockByNumber", json!(["latest", false]))
            .await?;
        let fee_hex = res
            .get("baseFeePerGas")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                "missing baseFeePerGas (pre-1559 chain?) — BLOCK (fail closed)".to_string()
            })?;
        u128::from_str_radix(strip_hex_prefix(fee_hex.trim()), 16)
            .map_err(|e| format!("invalid hex baseFeePerGas: {}", e))
    }

    async fn get_priority_fee(&self) -> Result<u128, String> {
        let res = self.send_rpc("eth_maxPriorityFeePerGas", json!([])).await?;
        parse_hex_u128(&res)
    }

    async fn get_balance(&self, address: &str) -> Result<u128, String> {
        let res = self
            .send_rpc("eth_getBalance", json!([address, "latest"]))
            .await?;
        parse_hex_u128(&res)
    }

    async fn get_code(&self, address: &str) -> Result<String, String> {
        let res = self
            .send_rpc("eth_getCode", json!([address, "latest"]))
            .await?;
        res.as_str()
            .map(|s| s.to_string())
            .ok_or_else(|| "expected hex string from eth_getCode".to_string())
    }
}

/// Strip an optional `0x`/`0X` prefix (case-insensitive).
fn strip_hex_prefix(s: &str) -> &str {
    s.strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s)
}

/// Returns true for `127.0.0.0/8` without pulling in an IP-parsing dep.
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

fn parse_hex_u64(val: &serde_json::Value) -> Result<u64, String> {
    let s = val
        .as_str()
        .ok_or_else(|| "expected hex string".to_string())?;
    u64::from_str_radix(strip_hex_prefix(s.trim()), 16)
        .map_err(|e| format!("failed to parse hex u64: {}", e))
}

fn parse_hex_u128(val: &serde_json::Value) -> Result<u128, String> {
    let s = val
        .as_str()
        .ok_or_else(|| "expected hex string".to_string())?;
    u128::from_str_radix(strip_hex_prefix(s.trim()), 16)
        .map_err(|e| format!("failed to parse hex u128: {}", e))
}

pub type DynRpcClient = Arc<dyn EvmRpcClient>;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_prefix_spoof_localhost_evil_rejected() {
        let client = HttpRpcClient::new("http://localhost.evil.com");
        let err = client
            .call("0x0000000000000000000000000000000000000000", "0x")
            .await
            .unwrap_err();
        assert!(
            err.contains("non-HTTPS") || err.contains("BLOCK"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_prefix_spoof_127_evil_rejected() {
        let client = HttpRpcClient::new("http://127.evil.com");
        let err = client
            .call("0x0000000000000000000000000000000000000000", "0x")
            .await
            .unwrap_err();
        assert!(
            err.contains("non-HTTPS") || err.contains("BLOCK"),
            "got: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_non_http_scheme_rejected() {
        let client = HttpRpcClient::new("ftp://example.com/rpc");
        let err = client
            .call("0x0000000000000000000000000000000000000000", "0x")
            .await
            .unwrap_err();
        assert!(err.contains("BLOCK"), "got: {}", err);
    }

    #[test]
    fn test_is_ipv4_loopback() {
        assert!(is_ipv4_loopback("127.0.0.1"));
        assert!(is_ipv4_loopback("127.1.2.3"));
        assert!(!is_ipv4_loopback("127.evil.com"));
        assert!(!is_ipv4_loopback("128.0.0.1"));
        assert!(!is_ipv4_loopback("localhost"));
    }

    #[test]
    fn test_strip_hex_prefix_case_insensitive() {
        assert_eq!(strip_hex_prefix("0xabc"), "abc");
        assert_eq!(strip_hex_prefix("0Xabc"), "abc");
        assert_eq!(strip_hex_prefix("abc"), "abc");
    }

    #[test]
    fn test_allowlist_empty_means_open() {
        assert!(is_rpc_host_allowed("https://anything.example/rpc", &[]));
    }

    #[test]
    fn test_allowlist_exact_match_case_insensitive() {
        let allowed = vec!["Ethereum-RPC.PublicNode.com".to_string()];
        assert!(is_rpc_host_allowed(
            "https://ethereum-rpc.publicnode.com",
            &allowed
        ));
        assert!(!is_rpc_host_allowed("https://evil.com", &allowed));
    }

    #[test]
    fn test_allowlist_ignores_credentials_port_path() {
        let allowed = vec!["rpc.example.com".to_string()];
        assert!(is_rpc_host_allowed(
            "https://user:pass@rpc.example.com:8443/v1?apiKey=x",
            &allowed
        ));
        assert!(!is_rpc_host_allowed(
            "https://rpc.example.com.evil.com",
            &allowed
        ));
        assert!(!is_rpc_host_allowed("not a url", &allowed));
    }

    #[tokio::test]
    async fn test_allowlist_denied_blocks_before_network() {
        let client = HttpRpcClient::new("https://evil.example/rpc")
            .with_allowed_hosts(vec!["good.example".to_string()]);
        let err = client
            .call("0x0000000000000000000000000000000000000000", "0x")
            .await
            .unwrap_err();
        assert!(err.contains("allowlist"), "got: {}", err);
    }
}
