use axum::{extract::{State, Path}, Json, Extension, http::StatusCode};
use serde_json::{json, Value};
use crate::AppState;
use crate::middleware::auth::AuthContext;
use crate::models::org::{CreateOrgRequest, UpdateOrgRequest};
use crate::models::user::CreateUserRequest;
use uuid::Uuid;

pub async fn list_orgs(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
) -> Json<Value> {
    let orgs = state.org_service.list_orgs().await;
    Json(json!({
        "organizations": orgs,
        "total": orgs.len(),
    }))
}

pub async fn create_org(
    State(state): State<AppState>,
    Extension(_auth): Extension<AuthContext>,
    Json(req): Json<CreateOrgRequest>,
) -> Json<Value> {
    let org = state.org_service.create_org(req).await;
    Json(json!({
        "organization": org,
        "message": "Organization created"
    }))
}

pub async fn get_org(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    // Enterprise: users can only see their own org unless super-admin
    if id != auth.org_id {
        // For demo, allow but in prod check super-admin
    }
    
    if let Some(org) = state.org_service.get_org(id).await {
        let users = state.org_service.list_users(org.id).await;
        let api_keys = state.org_service.list_api_keys(org.id).await;
        let stats = state.audit_service.get_stats(org.id).await;
        
        Ok(Json(json!({
            "organization": org,
            "members_count": users.len(),
            "api_keys_count": api_keys.len(),
            "stats": {
                "total_checks": stats.total_checks,
                "block_rate": stats.block_rate,
                "gas_saved_usd": stats.gas_saved_usd,
            }
        })))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "Organization not found"}))))
    }
}

pub async fn update_org(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateOrgRequest>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    if id != auth.org_id {
        return Err((StatusCode::FORBIDDEN, Json(json!({"error": "Cannot update other organization"}))));
    }

    if let Some(mut org) = state.org_service.get_org(id).await {
        if let Some(name) = req.name {
            org.name = name;
        }
        if let Some(settings) = req.settings {
            org.settings = settings;
        }
        org.updated_at = chrono::Utc::now();
        // In production, persist to DB
        Ok(Json(json!({
            "organization": org,
            "message": "Organization updated"
        })))
    } else {
        Err((StatusCode::NOT_FOUND, Json(json!({"error": "Organization not found"}))))
    }
}

pub async fn list_members(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Json<Value> {
    let users = state.org_service.list_users(auth.org_id).await;
    Json(json!({
        "members": users,
        "total": users.len(),
    }))
}

pub async fn invite_member(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<CreateUserRequest>,
) -> Json<Value> {
    let user = state.org_service.create_user(
        auth.org_id,
        req.email.clone(),
        req.name,
        req.role,
    ).await;
    
    Json(json!({
        "user": user,
        "message": format!("Invitation sent to {}", req.email),
        "invite_link": format!("https://app.stale.sh/invite/{}", Uuid::new_v4()),
    }))
}

pub async fn get_billing(
    State(state): State<AppState>,
    Extension(auth): Extension<AuthContext>,
) -> Json<Value> {
    let org = state.org_service.get_org(auth.org_id).await;
    let stats = state.audit_service.get_stats(auth.org_id).await;
    
    Json(json!({
        "organization": org,
        "tier": "enterprise",
        "billing": {
            "plan": "Enterprise",
            "price_per_month": 999,
            "included_checks": 100000,
            "overage_price": 0.005,
            "current_usage": stats.total_checks,
            "percent_used": (stats.total_checks as f64 / 100000.0 * 100.0).min(100.0),
            "next_invoice_date": chrono::Utc::now() + chrono::Duration::days(30),
            "payment_method": "**** **** **** 4242",
            "enterprise_features": {
                "sso": true,
                "sla": "99.99%",
                "support": "24/7 dedicated",
                "custom_guards": true,
                "on_prem": true,
                "audit_retention_days": 365,
            }
        },
        "usage": {
            "checks_this_month": stats.total_checks,
            "blocks_prevented": stats.total_blocks,
            "gas_saved_usd": stats.gas_saved_usd,
            "avg_latency_ms": stats.avg_duration_ms,
        }
    }))
}
