use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Owner,
    Admin,
    Developer,
    Auditor,
    Viewer,
}

impl Role {
    pub fn can_write_policies(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin | Role::Developer)
    }
    pub fn can_manage_team(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }
    pub fn can_manage_billing(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }
    pub fn can_view_audit(&self) -> bool {
        true // all roles can view, but filtered
    }
    pub fn can_manage_api_keys(&self) -> bool {
        matches!(self, Role::Owner | Role::Admin | Role::Developer)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub org_id: Uuid,
    pub email: String,
    pub name: String,
    pub role: Role,
    pub avatar_url: Option<String>,
    pub sso_id: Option<String>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub mfa_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Deserialize)]
pub struct InviteUserRequest {
    pub email: String,
    pub role: Role,
    pub message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub org_id: String,
    pub role: String,
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}
