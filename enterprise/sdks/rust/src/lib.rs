//! Stale Enterprise Rust SDK
//! Fail-closed guardrails for agents that touch money
//!
//! ```rust,no_run
//! use stale_enterprise_sdk::{Client, PipelineRequest};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = Client::new("stale_live_...", "https://api.stale.sh");
//!     let result = client.pipeline().run(PipelineRequest {
//!         rpc_url: "https://rpc.flashbots.net".to_string(),
//!         chain_id: Some(1),
//!         ..Default::default()
//!     }).await?;
//!     
//!     if result.is_block() {
//!         eprintln!("BLOCKED by {}: {}", result.blocked_by.unwrap_or_default(), result.reason);
//!         std::process::exit(1);
//!     }
//!     Ok(())
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StaleError {
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },
    #[error("Timeout after {0}ms - BLOCK (fail closed)")]
    Timeout(u64),
    #[error("Network error - BLOCK (fail closed): {0}")]
    Network(String),
    #[error("BLOCKED by {blocked_by}: {reason} (trace: {trace_id})")]
    Blocked { blocked_by: String, reason: String, trace_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRequest {
    pub rpc_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub environment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,
    #[serde(default)]
    pub checks: Vec<CheckRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl Default for PipelineRequest {
    fn default() -> Self {
        Self {
            rpc_url: String::new(),
            policy_id: None,
            environment: Some("production".to_string()),
            chain_id: None,
            checks: vec![],
            context: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckRequest {
    #[serde(rename = "type")]
    pub check_type: String,
    pub config: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardResult {
    pub guard_name: String,
    pub check_type: String,
    pub decision: String,
    pub reason: String,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResponse {
    pub trace_id: String,
    pub decision: String,
    pub blocked_by: Option<String>,
    pub reason: String,
    pub duration_ms: f64,
    pub results: Vec<GuardResult>,
    pub policy_id: Option<String>,
    pub policy_version: Option<u32>,
    pub audit_log_id: String,
}

impl PipelineResponse {
    pub fn is_allow(&self) -> bool {
        self.decision == "ALLOW"
    }
    pub fn is_block(&self) -> bool {
        self.decision == "BLOCK"
    }
}

pub struct Client {
    api_key: String,
    base_url: String,
    client: reqwest::Client,
}

impl Client {
    pub fn new(api_key: impl Into<String>, base_url: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap(),
        }
    }

    pub fn pipeline(&self) -> PipelineClient {
        PipelineClient { parent: self }
    }

    pub fn check(&self) -> CheckClient {
        CheckClient { parent: self }
    }

    pub fn audit(&self) -> AuditClient {
        AuditClient { parent: self }
    }
}

pub struct PipelineClient<'a> {
    parent: &'a Client,
}

impl<'a> PipelineClient<'a> {
    pub async fn run(&self, req: PipelineRequest) -> Result<PipelineResponse, StaleError> {
        let url = format!("{}/v1/pipeline/run", self.parent.base_url);
        let resp = self.parent.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.parent.api_key))
            .header("X-Stale-SDK", "rust/1.0.0")
            .json(&req)
            .send()
            .await
            .map_err(|e| StaleError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(StaleError::Http { status, message: text });
        }

        let result: PipelineResponse = resp.json().await
            .map_err(|e| StaleError::Network(e.to_string()))?;

        if result.is_block() {
            eprintln!("[stale] BLOCKED by {}: {} (trace: {})", 
                result.blocked_by.as_deref().unwrap_or("unknown"),
                result.reason,
                result.trace_id
            );
        }

        Ok(result)
    }

    pub async fn assert_allow(&self, req: PipelineRequest) -> Result<PipelineResponse, StaleError> {
        let result = self.run(req).await?;
        if result.is_block() {
            return Err(StaleError::Blocked {
                blocked_by: result.blocked_by.clone().unwrap_or_default(),
                reason: result.reason.clone(),
                trace_id: result.trace_id.clone(),
            });
        }
        Ok(result)
    }
}

pub struct CheckClient<'a> {
    parent: &'a Client,
}

impl<'a> CheckClient<'a> {
    pub async fn gas(&self, rpc_url: &str, max_gas_gwei: u64) -> Result<serde_json::Value, StaleError> {
        let url = format!("{}/v1/check/gas", self.parent.base_url);
        let resp = self.parent.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.parent.api_key))
            .json(&serde_json::json!({ "rpc_url": rpc_url, "max_gas_gwei": max_gas_gwei }))
            .send()
            .await
            .map_err(|e| StaleError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(StaleError::Http { status, message: text });
        }

        resp.json().await.map_err(|e| StaleError::Network(e.to_string()))
    }
}

pub struct AuditClient<'a> {
    parent: &'a Client,
}

impl<'a> AuditClient<'a> {
    pub async fn stats(&self) -> Result<serde_json::Value, StaleError> {
        let url = format!("{}/v1/audit/stats", self.parent.base_url);
        let resp = self.parent.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.parent.api_key))
            .send()
            .await
            .map_err(|e| StaleError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let text = resp.text().await.unwrap_or_default();
            return Err(StaleError::Http { status, message: text });
        }

        resp.json().await.map_err(|e| StaleError::Network(e.to_string()))
    }
}
