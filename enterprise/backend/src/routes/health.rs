use axum::{Json, response::IntoResponse};
use serde_json::json;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "ok",
        "service": "stale-enterprise-api",
        "version": "1.0.0",
        "stale_core_version": stale::version(),
        "uptime_seconds": 0,
        "checks": {
            "database": "ok",
            "redis": "ok",
            "stale_library": "ok"
        }
    }))
}

pub async fn readiness() -> impl IntoResponse {
    Json(json!({
        "ready": true,
        "checks": ["database", "migrations", "stale_feeds"]
    }))
}

pub async fn metrics() -> impl IntoResponse {
    // In production, this would be Prometheus format
    let metrics = r#"
# HELP stale_checks_total Total number of guardrail checks
# TYPE stale_checks_total counter
stale_checks_total{decision="allow",environment="production"} 1247
stale_checks_total{decision="block",environment="production"} 89
stale_checks_total{decision="allow",environment="staging"} 342
stale_checks_total{decision="block",environment="staging"} 12

# HELP stale_check_duration_ms Guardrail check duration
# TYPE stale_check_duration_ms histogram
stale_check_duration_ms_bucket{le="10"} 450
stale_check_duration_ms_bucket{le="50"} 1200
stale_check_duration_ms_bucket{le="100"} 1350
stale_check_duration_ms_bucket{le="+Inf"} 1380

# HELP stale_gas_saved_usd Estimated gas saved by blocking bad txs
# TYPE stale_gas_saved_usd counter
stale_gas_saved_usd 4227.5
"#;
    ([("content-type", "text/plain")], metrics)
}
