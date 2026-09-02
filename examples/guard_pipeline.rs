//! Example: Composable Pre-Flight Guard Pipeline for onchain AI agents
//! Run with: cargo run --example guard_pipeline

use stale::prelude::*;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🛡️ Initializing stale GuardPipeline...");

    // 1. Setup in-memory AuditLogger
    let audit = AuditLogger::default();

    // 2. Setup AddressBook
    let mut allowlist = HashMap::new();
    allowlist.insert(
        "UNISWAP_V3_ROUTER".to_string(),
        "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
    );
    let book = AddressBook::new(allowlist, true)?;

    // 3. Setup RateLimiter
    let mut rate_limiter = RateLimiter::new(10, 60)?;

    // 4. Build Pipeline
    let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, Some(audit));

    // Guard 1: Verify router is allowlisted
    let target = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    let check = book.check(target);
    pipeline.add("address_book", move || {
        let res = check.clone();
        async move { res }
    });

    // Guard 2: MEV-protected RPC enforcement
    pipeline.add("mev_protection", || async {
        check_mev_rpc("https://rpc.flashbots.net/fast")
    });

    // Guard 3: Rate limit check
    let rate_check = rate_limiter.check();
    pipeline.add("rate_limiter", move || {
        let res = rate_check.clone();
        async move { res }
    });

    // Run the pipeline
    let report = pipeline.run().await;

    println!("--------------------------------------------------");
    println!("Overall Decision : {}", report.decision);
    println!("Reason           : {}", report.reason);
    println!("Guards Run       : {}", report.guards_run);
    println!("Guards Passed    : {}", report.guards_passed);
    println!("Execution Time   : {:.2}ms", report.duration_ms);
    println!("--------------------------------------------------");

    for g in report.results {
        println!(
            "  - [{}] {}: {} ({:.2}ms)",
            g.decision, g.name, g.reason, g.duration_ms
        );
    }

    Ok(())
}
