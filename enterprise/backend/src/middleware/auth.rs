use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use crate::AppState;
use crate::models::api_key::ApiKeyScope;

#[derive(Clone, Debug)]
pub struct AuthContext {
    pub org_id: uuid::Uuid,
    pub user_id: Option<uuid::Uuid>,
    pub api_key_id: Option<uuid::Uuid>,
    pub scopes: Vec<ApiKeyScope>,
    pub environment: String,
    pub is_api_key: bool,
}

impl AuthContext {
    pub fn has_scope(&self, required: &ApiKeyScope) -> bool {
        self.scopes.contains(&ApiKeyScope::Admin) || self.scopes.contains(required)
    }

    pub fn enforce_scope(&self, required: ApiKeyScope) -> Result<(), StatusCode> {
        if self.has_scope(&required) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}

pub async fn api_key_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| {
            if s.starts_with("Bearer ") {
                Some(s.trim_start_matches("Bearer ").to_string())
            } else {
                Some(s.to_string())
            }
        })
        .or_else(|| {
            headers
                .get("x-api-key")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        });

    let raw_key = match api_key {
        Some(k) if !k.trim().is_empty() => k.trim().to_string(),
        _ => return Err(StatusCode::UNAUTHORIZED),
    };

    if let Some((api_key_obj, org)) = state.org_service.verify_api_key(&raw_key).await {
        if !api_key_obj.is_active {
            return Err(StatusCode::UNAUTHORIZED);
        }
        if let Some(expires_at) = api_key_obj.expires_at {
            if chrono::Utc::now() > expires_at {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }

        let auth_ctx = AuthContext {
            org_id: org.id,
            user_id: Some(api_key_obj.created_by),
            api_key_id: Some(api_key_obj.id),
            scopes: api_key_obj.scopes,
            environment: format!("{:?}", api_key_obj.environment).to_lowercase(),
            is_api_key: true,
        };
        request.extensions_mut().insert(auth_ctx);
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub fn enforce_scope(auth: &AuthContext, required: ApiKeyScope) -> Result<(), StatusCode> {
    auth.enforce_scope(required)
}
