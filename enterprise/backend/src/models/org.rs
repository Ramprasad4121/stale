use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub tier: OrgTier,
    pub status: OrgStatus,
    pub settings: OrgSettings,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrgTier {
    Free,
    Pro,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OrgStatus {
    Active,
    Suspended,
    Trial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgSettings {
    pub allowed_rpc_hosts: Vec<String>,
    pub default_max_age_seconds: u64,
    pub max_gas_gwei: u64,
    pub require_mev_protection: bool,
    pub enforce_allowlist: bool,
    pub webhook_url: Option<String>,
    pub slack_webhook_url: Option<String>,
    pub pagerduty_key: Option<String>,
    pub retention_days: u32,
    pub sso_enabled: bool,
    pub sso_provider: Option<String>, // okta, azure, google
}

impl Default for OrgSettings {
    fn default() -> Self {
        Self {
            allowed_rpc_hosts: vec![],
            default_max_age_seconds: 60,
            max_gas_gwei: 50,
            require_mev_protection: true,
            enforce_allowlist: true,
            webhook_url: None,
            slack_webhook_url: None,
            pagerduty_key: None,
            retention_days: 90,
            sso_enabled: false,
            sso_provider: None,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgRequest {
    pub name: String,
    pub slug: Option<String>,
    pub tier: Option<OrgTier>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateOrgRequest {
    pub name: Option<String>,
    pub settings: Option<OrgSettings>,
}
