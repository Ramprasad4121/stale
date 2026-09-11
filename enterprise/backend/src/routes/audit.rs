use axum::{extract::{State, Query}, Json, Extension, http::StatusCode};
use serde_json::{json, Value};
use crate::AppState;
use crate::middleware::auth::{AuthContext, enforce_scope};
use crate::models::audit::AuditQuery;
use crate::models::api_key::ApiKeyScope;
use uuid::Uuid;

pub async fn list_audit_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::AuditRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: audit_read"})))
    })?;

    let mut q = query;
    q.org_id = Some(auth.org_id);
    
    let logs = state.audit_service.query(q).await;
    
    Ok(Json(json!({
        "logs": logs,
        "total": logs.len(),
        "org_id": auth.org_id,
    })))
}

pub async fn get_audit_stats(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::AuditRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: audit_read"})))
    })?;

    let stats = state.audit_service.get_stats(auth.org_id).await;
    Ok(Json(json!(stats)))
}

pub async fn get_audit_log(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::AuditRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: audit_read"})))
    })?;

    if let Some(log) = state.audit_service.get_by_id(id).await {
        if log.org_id != auth.org_id {
            return Err((
                axum::http::StatusCode::FORBIDDEN,
                Json(json!({"error": "Access denied"}))
            ));
        }
        Ok(Json(json!(log)))
    } else {
        Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(json!({"error": "Audit log not found"}))
        ))
    }
}

pub async fn export_audit_logs(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<AuditQuery>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    enforce_scope(&auth, ApiKeyScope::AuditRead).map_err(|_| {
        (StatusCode::FORBIDDEN, Json(json!({"error": "Missing scope: audit_read"})))
    })?;

    let mut q = query;
    q.org_id = Some(auth.org_id);
    q.limit = Some(10000);
    
    let logs = state.audit_service.query(q).await;
    
    Ok(Json(json!({
        "export_id": Uuid::new_v4(),
        "status": "completed",
        "format": "json",
        "count": logs.len(),
        "download_url": format!("/v1/audit/exports/{}.json", Uuid::new_v4()),
        "expires_at": chrono::Utc::now() + chrono::Duration::hours(24),
        "message": "In production, this would upload to S3 and provide signed URL. For demo, logs included inline.",
        "logs": logs,
    })))
}
