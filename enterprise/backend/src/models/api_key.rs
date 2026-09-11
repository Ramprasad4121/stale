use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyScope {
    CheckRead,
    CheckWrite,
    PipelineRun,
    AuditRead,
    PoliciesRead,
    PoliciesWrite,
    Admin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub org_id: Uuid,
    pub name: String,
    pub key_prefix: String, // first 8 chars visible
    pub key_hash: String,   // bcrypt hash of full key
    pub scopes: Vec<ApiKeyScope>,
    pub environment: Environment,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub rate_limit_per_min: u32,
    pub ip_allowlist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Production,
    Staging,
    Development,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub scopes: Vec<ApiKeyScope>,
    pub environment: Environment,
    pub expires_in_days: Option<u32>,
    pub rate_limit_per_min: Option<u32>,
    pub ip_allowlist: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub api_key: String, // only returned once!
    pub key_prefix: String,
    pub scopes: Vec<ApiKeyScope>,
    pub environment: Environment,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ApiKeyAuth {
    pub org_id: Uuid,
    pub key_id: Uuid,
    pub scopes: Vec<ApiKeyScope>,
    pub environment: Environment,
}
