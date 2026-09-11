mod models;
mod services;
mod routes;
mod middleware;

use axum::{
    routing::{get, post},
    Router,
    middleware as axum_middleware,
};
use std::net::SocketAddr;
use tower_http::{cors::{CorsLayer, Any}, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use services::{StaleService, AuditService, PolicyService, OrgService, WebhookService};

#[derive(Clone)]
pub struct AppState {
    pub stale_service: StaleService,
    pub audit_service: AuditService,
    pub policy_service: PolicyService,
    pub org_service: OrgService,
    pub webhook_service: WebhookService,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "stale_enterprise_api=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    println!(r#"
  ███████╗████████╗ █████╗ ██╗     ███████╗
  ██╔════╝╚══██╔══╝██╔══██╗██║     ██╔════╝
  ███████╗   ██║   ███████║██║     █████╗  
  ╚════██║   ██║   ██╔══██║██║     ██╔══╝  
  ███████║   ██║   ██║  ██║███████╗███████╗
  ╚══════╝   ╚═╝   ╚═╝  ╚═╝╚══════╝╚══════╝  
                                           
  Enterprise Guardrail Platform v1.0.0
  Fail-closed security for agents that touch money
"#);

    let stale_service = StaleService::new();
    let audit_service = AuditService::new();
    let policy_service = PolicyService::new();
    let org_service = OrgService::new();
    let webhook_service = WebhookService::new();

    println!("🌱 Seeding demo enterprise data...");
    let (org, user) = org_service.seed_demo().await;
    println!("✅ Org: {} ({})", org.name, org.id);
    println!("✅ User: {} ({})", user.name, user.email);
    
    policy_service.seed_demo(org.id, user.id).await;
    audit_service.seed_demo(org.id).await;
    println!("✅ Policies and audit logs seeded");
    println!();

    let state = AppState {
        stale_service,
        audit_service,
        policy_service,
        org_service,
        webhook_service,
    };

    let public_routes = Router::new()
        .route("/health", get(routes::health_check))
        .route("/ready", get(routes::readiness))
        .route("/metrics", get(routes::metrics));

    let protected_routes = Router::new()
        .route("/v1/check/price", post(routes::check_price))
        .route("/v1/check/gas", post(routes::check_gas))
        .route("/v1/check/sequencer", post(routes::check_sequencer))
        .route("/v1/check/mev", post(routes::check_mev))
        .route("/v1/check/chain", post(routes::check_chain))
        .route("/v1/pipeline/run", post(routes::run_pipeline))
        .route("/v1/pipeline/status", get(routes::get_pipeline_status))
        .route("/v1/audit/logs", get(routes::list_audit_logs))
        .route("/v1/audit/stats", get(routes::get_audit_stats))
        .route("/v1/audit/logs/:id", get(routes::get_audit_log))
        .route("/v1/audit/export", get(routes::export_audit_logs))
        .route("/v1/policies", get(routes::list_policies).post(routes::create_policy))
        .route("/v1/policies/:id", get(routes::get_policy).patch(routes::update_policy).delete(routes::delete_policy))
        .route("/v1/policies/:id/versions", get(routes::get_policy_versions))
        .route("/v1/orgs", get(routes::list_orgs).post(routes::create_org))
        .route("/v1/orgs/:id", get(routes::get_org).patch(routes::update_org))
        .route("/v1/orgs/members", get(routes::list_members))
        .route("/v1/orgs/members/invite", post(routes::invite_member))
        .route("/v1/orgs/billing", get(routes::get_billing))
        .route("/v1/api-keys", get(routes::list_api_keys).post(routes::create_api_key))
        .route("/v1/api-keys/:id/revoke", post(routes::revoke_api_key))
        .route("/v1/api-keys/:id/rotate", post(routes::rotate_api_key))
        .route("/v1/feeds", get(routes::list_feeds))
        .route("/v1/feeds/:symbol", get(routes::get_feed))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            middleware::auth::api_key_auth,
        ));

    // CORS: In production, lock down to specific origins (not Any)
    // For demo, we allow specific origins, but prod should use allow_origin with explicit list
    let cors = if std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()) == "production" {
        // Production: only allow app.stale.sh and localhost for docs
        CorsLayer::new()
            .allow_origin([
                "https://app.stale.sh".parse::<axum::http::HeaderValue>().unwrap(),
                "https://api.stale.sh".parse::<axum::http::HeaderValue>().unwrap(),
            ])
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PATCH, axum::http::Method::DELETE])
            .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE, axum::http::header::HeaderName::from_static("x-api-key")])
    } else {
        // Development: more permissive but still not Any for methods/headers
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PATCH, axum::http::Method::DELETE])
            .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE, axum::http::header::HeaderName::from_static("x-api-key")])
    };

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string()).parse::<u16>().unwrap_or(3001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    
    println!("🚀 Stale Enterprise API running on http://{}", addr);
    println!();
    println!("📚 Endpoints:");
    println!("   Health:        GET  /health");
    println!("   Check Price:   POST /v1/check/price");
    println!("   Run Pipeline:  POST /v1/pipeline/run");
    println!("   Audit Logs:    GET  /v1/audit/logs");
    println!("   Policies:      GET  /v1/policies");
    println!("   Feeds:         GET  /v1/feeds");
    println!();
    println!("🔐 Auth: Requires API key - Authorization: Bearer <key> or X-API-Key: <key>");
    println!("   Demo org ID: {} - list keys via GET /v1/api-keys with valid key", org.id);
    println!();
    println!("💡 Example (with valid API key):");
    println!("   curl http://localhost:{}/v1/feeds -H 'Authorization: Bearer stale_live_...'", port);
    println!();

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
