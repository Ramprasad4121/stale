use axum::{extract::{State, Path, Query}, Json, Extension, http::StatusCode};
use serde_json::{json, Value};
use crate::AppState;
use crate::middleware::auth::{AuthContext, enforce_scope};
use crate::models::policy::{CreatePolicyRequest, UpdatePolicyRequest};
use crate::models::api_key::ApiKeyScope;
use uuid::Uuid;
use std::collections::HashMap;

pub async fn list_policies(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PoliciesRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: policies_read"})))
    })?;

    let env = params.get("environment").cloned();
    let policies = state.policy_service.list(auth.org_id, env).await;
    Ok(Json(json!({
        "policies": policies,
        "total": policies.len(),
    })))
}

pub async fn create_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreatePolicyRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PoliciesWrite).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: policies_write"})))
    })?;

    if req.name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"error": "Policy name required"}))
        ));
    }

    let user_id = auth.user_id.unwrap_or_else(|| Uuid::new_v4());
    let policy = state.policy_service.create(auth.org_id, user_id, req).await;
    
    Ok(Json(json!({
        "policy": policy,
        "message": "Policy created successfully"
    })))
}

pub async fn get_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PoliciesRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: policies_read"})))
    })?;

    if let Some(policy) = state.policy_service.get(id).await {
        if policy.org_id != auth.org_id {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Access denied"}))));
        }
        Ok(Json(json!({"policy": policy})))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "Policy not found"}))))
    }
}

pub async fn update_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdatePolicyRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PoliciesWrite).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: policies_write"})))
    })?;

    if let Some(existing) = state.policy_service.get(id).await {
        if existing.org_id != auth.org_id {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Access denied"}))));
        }
    }

    if let Some(policy) = state.policy_service.update(id, req).await {
        Ok(Json(json!({
            "policy": policy,
            "message": "Policy updated - new version created"
        })))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "Policy not found"}))))
    }
}

pub async fn delete_policy(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PoliciesWrite).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: policies_write"})))
    })?;

    if let Some(existing) = state.policy_service.get(id).await {
        if existing.org_id != auth.org_id {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Access denied"}))));
        }
        if existing.is_default {
            return Err((StatusCode::BAD_REQUEST, Json(json!({"error": "Cannot delete default policy - set another as default first"}))));
        }
    }

    if state.policy_service.delete(id).await {
        Ok(Json(json!({"message": "Policy deleted"})))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "Policy not found"}))))
    }
}

pub async fn get_policy_versions(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::PoliciesRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: policies_read"})))
    })?;

    if let Some(policy) = state.policy_service.get(id).await {
        if policy.org_id != auth.org_id {
            return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Access denied"}))));
        }
        Ok(Json(json!({
            "policy_id": id,
            "current_version": policy.version,
            "versions": [
                {
                    "version": policy.version,
                    "created_at": policy.updated_at,
                    "created_by": policy.created_by,
                    "changes": "Current version",
                    "rules": policy.rules,
                },
                {
                    "version": policy.version - 1,
                    "created_at": policy.created_at,
                    "created_by": policy.created_by,
                    "changes": "Initial version",
                    "rules": policy.rules,
                }
            ]
        })))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "Policy not found"}))))
    }
}
