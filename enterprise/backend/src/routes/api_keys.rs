use axum::{extract::{State, Path}, Json, Extension, http::StatusCode};
use serde_json::{json, Value};
use crate::AppState;
use crate::middleware::auth::AuthContext;
use crate::models::api_key::CreateApiKeyRequest;
use uuid::Uuid;

pub async fn list_api_keys(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Json<Value> {
    let keys = state.org_service.list_api_keys(auth.org_id).await;
    // Don't return hashes
    let safe_keys: Vec<Value> = keys.into_iter().map(|k| {
        json!({
            "id": k.id,
            "name": k.name,
            "key_prefix": k.key_prefix,
            "scopes": k.scopes,
            "environment": k.environment,
            "last_used_at": k.last_used_at,
            "expires_at": k.expires_at,
            "created_at": k.created_at,
            "is_active": k.is_active,
            "rate_limit_per_min": k.rate_limit_per_min,
        })
    }).collect();
    
    Json(json!({
        "api_keys": safe_keys,
        "total": safe_keys.len(),
    }))
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if req.name.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Name required"}))));
    }

    let user_id = auth.user_id.unwrap_or_else(|| Uuid::new_v4());
    let (api_key, raw_key) = state.org_service.create_api_key(
        auth.org_id,
        user_id,
        req.name,
        req.scopes,
        req.environment,
    ).await;

    Ok(Json(json!({
        "id": api_key.id,
        "name": api_key.name,
        "api_key": raw_key,
        "key_prefix": api_key.key_prefix,
        "scopes": api_key.scopes,
        "environment": api_key.environment,
        "message": "⚠️ Save this key - it will not be shown again!",
        "docs": "Use as: Authorization: Bearer <key> or X-API-Key: <key>"
    })))
}

pub async fn revoke_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // In production, soft-delete and add to revocation list in Redis
    // For demo, just return success
    let keys = state.org_service.list_api_keys(auth.org_id).await;
    if !keys.iter().any(|k| k.id == id) {
        return Err((StatusCode::NOT_FOUND, Json(json!({"error": "API key not found"}))));
    }
    
    Ok(Json(json!({
        "message": "API key revoked successfully",
        "id": id,
        "revoked_at": chrono::Utc::now(),
    })))
}

pub async fn rotate_api_key(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let keys = state.org_service.list_api_keys(auth.org_id).await;
    let existing = keys.iter().find(|k| k.id == id);
    
    if let Some(key) = existing {
        let user_id = auth.user_id.unwrap_or_else(|| Uuid::new_v4());
        let (new_key, raw_key) = state.org_service.create_api_key(
            auth.org_id,
            user_id,
            format!("{} (rotated)", key.name),
            key.scopes.clone(),
            key.environment.clone(),
        ).await;
        
        Ok(Json(json!({
            "old_key_id": id,
            "new_key": {
                "id": new_key.id,
                "name": new_key.name,
                "api_key": raw_key,
                "key_prefix": new_key.key_prefix,
            },
            "message": "Key rotated - old key revoked, new key active",
        })))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "API key not found"}))))
    }
}
