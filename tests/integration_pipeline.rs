use stale::prelude::*;
use std::collections::HashMap;

#[tokio::test]
async fn test_full_guardrail_preflight_pipeline() {
    let audit = AuditLogger::default();

    // 1. Configure AddressBook
    let mut allowlist = HashMap::new();
    allowlist.insert(
        "UNISWAP_V3_ROUTER".to_string(),
        "0xE592427A0AEce92De3Edee1F18E0157C05861564".to_string(),
    );
    let book = AddressBook::new(allowlist, true).unwrap();

    // 2. Configure RateLimiter & SpendingCap
    let mut rate_limiter = RateLimiter::new(10, 60).unwrap();
    let mut spending_cap = SpendingCap::new(5_000_000_000_000_000_000, 60).unwrap(); // 5 ETH

    // 3. Assemble Pipeline
    let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, Some(audit.clone()));

    // Guard A: Target is in AddressBook
    let router_addr = "0xE592427A0AEce92De3Edee1F18E0157C05861564";
    let book_check = book.check(router_addr);
    pipeline.add("address_book", move || {
        let res = book_check.clone();
        async move { res }
    });

    // Guard B: Deadline is valid
    let now = 1700000000;
    let deadline_check = check_deadline(
        CheckDeadlineInput {
            deadline: now + 300,
            max_future_seconds: Some(1200),
            min_future_seconds: Some(30),
        },
        now,
    );
    pipeline.add("deadline", move || {
        let res = deadline_check.clone();
        async move { res }
    });

    // Guard C: Slippage bound is computed
    let slippage_res = calculate_min_amount_out(CalculateMinAmountOutInput {
        amount_in: 1_000_000_000_000_000_000,
        token_in_decimals: 18,
        price_in_answer: 2500_00000000,
        price_in_decimals: 8,
        token_out_decimals: 6,
        price_out_answer: 1_00000000,
        price_out_decimals: 8,
        slippage_bps: 50,
    });
    pipeline.add("slippage", move || {
        let res = match &slippage_res {
            Ok(min_out) => GuardrailResult::allow(format!("min_amount_out: {}", min_out)),
            Err(e) => GuardrailResult::block(e.clone()),
        };
        async move { res }
    });

    // Guard D: MEV Protected RPC
    let mev_check = check_mev_rpc("https://rpc.flashbots.net/fast");
    pipeline.add("mev_rpc", move || {
        let res = mev_check.clone();
        async move { res }
    });

    // Guard E: Rate Limiter
    let rate_check = rate_limiter.check();
    pipeline.add("rate_limit", move || {
        let res = rate_check.clone();
        async move { res }
    });

    // Guard F: Spending Cap
    let spend_check = spending_cap.check(1_000_000_000_000_000_000);
    pipeline.add("spending_cap", move || {
        let res = spend_check.clone();
        async move { res }
    });

    // Execute pipeline
    let report = pipeline.run().await;

    assert_eq!(report.decision, Decision::Allow);
    assert_eq!(report.guards_run, 6);
    assert_eq!(report.guards_passed, 6);
    assert!(report.blocked_by.is_none());
}
