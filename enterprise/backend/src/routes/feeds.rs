use axum::{Json, Extension};
use serde_json::{json, Value};
use crate::middleware::auth::AuthContext;
use stale::feeds::{FEEDS, REGISTRY, DEFAULT_FEED};

pub async fn list_feeds(
    Extension(_auth): Extension<AuthContext>,
) -> Json<Value> {
    let feeds: Vec<Value> = FEEDS.iter().map(|f| {
        json!({
            "symbol": f.symbol,
            "address": f.address,
            "chain_id": f.chain_id,
            "chain_name": match f.chain_id {
                1 => "ethereum",
                10 => "optimism",
                137 => "polygon",
                8453 => "base",
                42161 => "arbitrum",
                324 => "zksync",
                1088 => "metis",
                5000 => "mantle",
                534352 => "scroll",
                _ => "unknown"
            },
            "description": format!("{} Chainlink Feed on chain {}", f.symbol, f.chain_id),
        })
    }).collect();

    Json(json!({
        "feeds": feeds,
        "total": feeds.len(),
        "default_feed": DEFAULT_FEED,
        "registry_size": REGISTRY.len(),
        "supported_chains": [
            {"id": 1, "name": "ethereum"},
            {"id": 10, "name": "optimism"},
            {"id": 137, "name": "polygon"},
            {"id": 8453, "name": "base"},
            {"id": 42161, "name": "arbitrum"},
            {"id": 324, "name": "zksync"},
            {"id": 1088, "name": "metis"},
            {"id": 5000, "name": "mantle"},
            {"id": 534352, "name": "scroll"}
        ],
    }))
}

pub async fn get_feed(
    Extension(_auth): Extension<AuthContext>,
    axum::extract::Path(symbol): axum::extract::Path<String>,
) -> Json<Value> {
    let lookup_key = symbol.to_uppercase();
    let found = FEEDS.iter().find(|f| {
        f.symbol.to_uppercase() == lookup_key || 
        f.address.to_lowercase() == symbol.to_lowercase()
    });

    if let Some(feed) = found {
        Json(json!({
            "symbol": feed.symbol,
            "address": feed.address,
            "chain_id": feed.chain_id,
            "found": true,
        }))
    } else {
        Json(json!({
            "symbol": symbol,
            "found": false,
            "message": format!("Feed {} not found - check /v1/feeds for available", symbol),
            "suggestion": "Use ETH/USD, BTC/USD, USDC/USD etc",
        }))
    }
}
