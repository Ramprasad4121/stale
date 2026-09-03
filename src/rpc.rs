use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

#[async_trait]
pub trait EvmRpcClient: Send + Sync {
    async fn call(&self, to: &str, data: &str) -> Result<String, String>;
    async fn call_from(&self, from: &str, to: &str, data: &str) -> Result<String, String>;
    async fn get_block_number(&self) -> Result<u64, String>;
    async fn get_block_timestamp(&self) -> Result<u64, String>;
    async fn get_chain_id(&self) -> Result<u64, String>;
    async fn get_transaction_count(&self, address: &str) -> Result<u64, String>;
    async fn get_gas_price(&self) -> Result<u128, String>;
    async fn get_balance(&self, address: &str) -> Result<u128, String>;
    async fn get_code(&self, address: &str) -> Result<String, String>;
}

#[derive(Clone)]
pub struct HttpRpcClient {
    rpc_url: String,
    client: reqwest::Client,
}

impl HttpRpcClient {
    pub fn new(rpc_url: impl Into<String>) -> Self {
        let rpc_url = rpc_url.into();
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self { rpc_url, client }
    }

    fn validate_url(&self) -> Result<(), String> {
        let lower = self.rpc_url.to_lowercase();
        if lower.starts_with("https://") {
            return Ok(());
        }
        if lower.starts_with("http://localhost")
            || lower.starts_with("http://127.")
            || lower.starts_with("http://[::1]")
        {
            return Ok(());
        }
        Err("refusing non-HTTPS RPC URL (MITM risk) — BLOCK. Use https:// or local http://localhost".to_string())
    }

    /// Strip the configured RPC URL (which may embed an API key) from error
    /// text so secrets never land in GuardrailResult.reason / audit logs.
    fn redact(&self, msg: String) -> String {
        if self.rpc_url.is_empty() {
            return msg;
        }
        msg.replace(&self.rpc_url, "<rpc-url>")
    }

    async fn send_rpc(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.validate_url()?;
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

        let body: serde_json::Value = resp
            .json()
            .await
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
        u64::from_str_radix(ts_hex.trim_start_matches("0x"), 16)
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

fn parse_hex_u64(val: &serde_json::Value) -> Result<u64, String> {
    let s = val
        .as_str()
        .ok_or_else(|| "expected hex string".to_string())?;
    u64::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| format!("failed to parse hex u64: {}", e))
}

fn parse_hex_u128(val: &serde_json::Value) -> Result<u128, String> {
    let s = val
        .as_str()
        .ok_or_else(|| "expected hex string".to_string())?;
    u128::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| format!("failed to parse hex u128: {}", e))
}

pub type DynRpcClient = Arc<dyn EvmRpcClient>;
