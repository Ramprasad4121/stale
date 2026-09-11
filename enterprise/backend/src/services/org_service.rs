use crate::models::org::{Organization, OrgTier, OrgStatus, OrgSettings, CreateOrgRequest};
use crate::models::user::{User, Role};
use crate::models::api_key::{ApiKey, ApiKeyScope, Environment};
use chrono::Utc;
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct OrgService {
    orgs: Arc<DashMap<Uuid, Organization>>,
    users: Arc<DashMap<Uuid, User>>,
    api_keys: Arc<DashMap<Uuid, ApiKey>>,
}

impl OrgService {
    pub fn new() -> Self {
        Self {
            orgs: Arc::new(DashMap::new()),
            users: Arc::new(DashMap::new()),
            api_keys: Arc::new(DashMap::new()),
        }
    }

    pub async fn create_org(&self, req: CreateOrgRequest) -> Organization {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let slug = req.slug.unwrap_or_else(|| req.name.to_lowercase().replace(' ', "-"));
        
        let org = Organization {
            id,
            name: req.name,
            slug,
            tier: req.tier.unwrap_or(OrgTier::Enterprise),
            status: OrgStatus::Active,
            settings: OrgSettings::default(),
            created_at: now,
            updated_at: now,
        };
        
        self.orgs.insert(id, org.clone());
        org
    }

    pub async fn get_org(&self, id: Uuid) -> Option<Organization> {
        self.orgs.get(&id).map(|e| e.value().clone())
    }

    pub async fn list_orgs(&self) -> Vec<Organization> {
        self.orgs.iter().map(|e| e.value().clone()).collect()
    }

    pub async fn create_user(&self, org_id: Uuid, email: String, name: String, role: Role) -> User {
        let id = Uuid::new_v4();
        let user = User {
            id,
            org_id,
            email,
            name,
            role,
            avatar_url: None,
            sso_id: None,
            last_login_at: Some(Utc::now()),
            created_at: Utc::now(),
            mfa_enabled: true,
        };
        self.users.insert(id, user.clone());
        user
    }

    pub async fn get_user(&self, id: Uuid) -> Option<User> {
        self.users.get(&id).map(|e| e.value().clone())
    }

    pub async fn list_users(&self, org_id: Uuid) -> Vec<User> {
        self.users.iter()
            .filter(|e| e.value().org_id == org_id)
            .map(|e| e.value().clone())
            .collect()
    }

    pub async fn create_api_key(
        &self,
        org_id: Uuid,
        created_by: Uuid,
        name: String,
        scopes: Vec<ApiKeyScope>,
        env: Environment,
    ) -> (ApiKey, String) {
        let id = Uuid::new_v4();
        let raw_key = format!("stale_{}_{}", 
            match env {
                Environment::Production => "live",
                Environment::Staging => "test",
                Environment::Development => "dev",
            },
            hex::encode(Uuid::new_v4().as_bytes())
        );
        let prefix = raw_key.chars().take(12).collect::<String>();
        let hash = bcrypt::hash(&raw_key, 10).unwrap_or_else(|_| {
            bcrypt::hash(&raw_key, 4).unwrap_or_else(|_| raw_key.clone())
        });

        let api_key = ApiKey {
            id,
            org_id,
            name,
            key_prefix: prefix,
            key_hash: hash,
            scopes,
            environment: env,
            last_used_at: None,
            expires_at: None,
            created_by,
            created_at: Utc::now(),
            is_active: true,
            rate_limit_per_min: 1000,
            ip_allowlist: vec![],
        };

        self.api_keys.insert(id, api_key.clone());
        (api_key, raw_key)
    }

    pub async fn verify_api_key(&self, raw_key: &str) -> Option<(ApiKey, Organization)> {
        if raw_key.trim().is_empty() {
            return None;
        }

        for entry in self.api_keys.iter() {
            let stored = entry.value();
            if !stored.is_active {
                continue;
            }

            if let Some(expires_at) = stored.expires_at {
                if Utc::now() > expires_at {
                    continue;
                }
            }

            let is_valid = bcrypt::verify(raw_key, &stored.key_hash).unwrap_or(false);

            if is_valid {
                if let Some(org) = self.orgs.get(&stored.org_id) {
                    return Some((stored.clone(), org.clone()));
                }
            }
        }
        
        None
    }

    pub async fn list_api_keys(&self, org_id: Uuid) -> Vec<ApiKey> {
        self.api_keys.iter()
            .filter(|e| e.value().org_id == org_id)
            .map(|e| e.value().clone())
            .collect()
    }

    pub async fn seed_demo(&self) -> (Organization, User) {
        let org = self.create_org(CreateOrgRequest {
            name: "Acme DeFi Corp".to_string(),
            slug: Some("acme-defi".to_string()),
            tier: Some(OrgTier::Enterprise),
        }).await;

        let user = self.create_user(
            org.id,
            "ramprasad@acme.defi".to_string(),
            "Ramprasad Goud".to_string(),
            Role::Owner,
        ).await;

        let _ = self.create_user(org.id, "alice@acme.defi".to_string(), "Alice Chen".to_string(), Role::Admin).await;
        let _ = self.create_user(org.id, "bob@acme.defi".to_string(), "Bob Nakamoto".to_string(), Role::Developer).await;
        let _ = self.create_user(org.id, "carol@acme.defi".to_string(), "Carol Security".to_string(), Role::Auditor).await;

        // Create demo API keys - raw keys are NOT logged (CodeQL cleartext-logging fix)
        // Only returned via secure API creation endpoint once
        let (_prod_key, _raw_prod) = self.create_api_key(
            org.id,
            user.id,
            "Production - Trading Bot".to_string(),
            vec![ApiKeyScope::CheckRead, ApiKeyScope::CheckWrite, ApiKeyScope::PipelineRun, ApiKeyScope::AuditRead],
            Environment::Production,
        ).await;

        let (_staging_key, _) = self.create_api_key(
            org.id,
            user.id,
            "Staging - CI/CD".to_string(),
            vec![ApiKeyScope::CheckRead, ApiKeyScope::PipelineRun],
            Environment::Staging,
        ).await;

        // SECURITY: Do NOT log API key material or key-derived metadata (prefix, ID)
        // CodeQL flags key_prefix as sensitive - omit entirely from logs
        // Raw keys are returned via API creation endpoint only once
        println!("✅ Seeded org: {} ({})", org.name, org.id);
        println!("✅ Seeded demo API keys (details omitted from logs for security)");
        println!("   Note: API key values and identifiers are not logged. Use secure API key management endpoints.");

        (org, user)
    }
}

impl Default for OrgService {
    fn default() -> Self {
        Self::new()
    }
}
