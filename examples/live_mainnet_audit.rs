//! Live Real-World Mainnet Audit & Verification Tool
//! Run with: cargo run --example live_mainnet_audit

use stale::prelude::*;
use stale::{check_sequencer, DEFAULT_FEED};
use std::time::Duration;
use tokio::time::sleep;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mainnet_rpc = HttpRpcClient::new("https://ethereum-rpc.publicnode.com");
    let arb_rpc = HttpRpcClient::new("https://arb1.arbitrum.io/rpc");

    println!("============================================================");
    println!("🛡️  STALE REAL-WORLD ON-CHAIN AUDIT & VERIFICATION SUITE");
    println!("============================================================");

    // 1. Live Chainlink Feed
    println!("\n[1] Testing Live Chainlink ETH/USD Feed (Mainnet)...");
    let feed_res = check_price(
        &mainnet_rpc,
        CheckPriceInput {
            feed: DEFAULT_FEED,
            max_age_seconds: 86400,
            amount_eth: Some(1.0),
            now_seconds: None,
        },
    )
    .await;
    println!("  -> Decision: {}", feed_res.decision);
    println!("  -> Price   : ${:.2}", feed_res.price_usd.unwrap_or(0.0));
    println!("  -> Reason  : {}", feed_res.reason);
    assert_eq!(feed_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 2. Live Uniswap V3 Pool
    println!("\n[2] Testing Live Uniswap V3 USDC/WETH Liquidity (0.05%)...");
    let v3_res = check_pool_v3(
        &mainnet_rpc,
        "0x88e6A0c2dDD26FEEb64F039a2c41296FcB3f5640",
        1_000_000,
    )
    .await;
    println!("  -> Decision: {}", v3_res.decision);
    println!("  -> Reason  : {}", v3_res.reason);
    assert_eq!(v3_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 3. Live Uniswap V2 Pool
    println!("\n[3] Testing Live Uniswap V2 USDC/WETH Reserves...");
    let v2_res = check_pool_v2(
        &mainnet_rpc,
        "0xB4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc",
        1_000_000,
        1_000_000,
    )
    .await;
    println!("  -> Decision: {}", v2_res.decision);
    println!("  -> Reason  : {}", v2_res.reason);
    assert_eq!(v2_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 4. Live OFAC Sanctions Oracle (Vitalik vs Lazarus Hacker)
    println!("\n[4] Testing Live OFAC Sanctions Oracle...");
    let clean_res = check_sanctioned(
        &mainnet_rpc,
        1,                                            // Ethereum mainnet: sanctions oracle chain
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // Vitalik
    )
    .await;
    println!("  -> Vitalik Address  : {}", clean_res.decision);
    println!("  -> Vitalik Reason   : {}", clean_res.reason);
    assert_eq!(clean_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    let lazarus_res = check_sanctioned(
        &mainnet_rpc,
        1,                                            // Ethereum mainnet: sanctions oracle chain
        "0x098B716B8Aaf21512996dC57EB0615e2383E2f96", // Lazarus Group
    )
    .await;
    println!("  -> Lazarus Address  : {}", lazarus_res.decision);
    println!("  -> Lazarus Reason   : {}", lazarus_res.reason);
    assert_eq!(lazarus_res.decision, Decision::Block);
    sleep(Duration::from_millis(200)).await;

    // 5. Live EOA vs Smart Contract Check
    println!("\n[5] Testing Bytecode EOA Phishing Check...");
    let eoa_res = check_is_contract(
        &mainnet_rpc,
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045", // Vitalik EOA
    )
    .await;
    println!("  -> EOA Decision     : {}", eoa_res.decision);
    assert_eq!(eoa_res.decision, Decision::Block);
    sleep(Duration::from_millis(200)).await;

    let contract_res = check_is_contract(
        &mainnet_rpc,
        "0xE592427A0AEce92De3Edee1F18E0157C05861564", // Uniswap V3 Router
    )
    .await;
    println!("  -> Contract Decision: {}", contract_res.decision);
    assert_eq!(contract_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 6. Live Protocol Pause Check on USDC
    println!("\n[6] Testing Live Protocol Pause State (USDC)...");
    let pause_res = check_paused(
        &mainnet_rpc,
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", // USDC
    )
    .await;
    println!("  -> USDC Pause Status: {}", pause_res.decision);
    println!("  -> Reason           : {}", pause_res.reason);
    assert_eq!(pause_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 7. Live Solvency Check on Vitalik ETH Balance
    println!("\n[7] Testing Live Native ETH Solvency...");
    let solv_res = check_balance(
        &mainnet_rpc,
        "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
        None,
        100_000_000_000_000_000, // 0.1 ETH
        1_000_000_000_000_000,   // 0.001 ETH gas reserve
    )
    .await;
    println!("  -> Solvency Status  : {}", solv_res.decision);
    println!("  -> Reason           : {}", solv_res.reason);
    assert_eq!(solv_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 8. Live Network Gas Check
    println!("\n[8] Testing Live Basefee / Gas Circuit Breaker...");
    let gas_res = check_gas_price(&mainnet_rpc, 100).await; // 100 Gwei limit
    println!("  -> Gas Price Status : {}", gas_res.decision);
    println!("  -> Reason           : {}", gas_res.reason);
    assert_eq!(gas_res.decision, Decision::Allow);
    sleep(Duration::from_millis(200)).await;

    // 9. Live Arbitrum Sequencer Uptime Feed on Arbitrum One
    println!("\n[9] Testing Live Arbitrum Sequencer Uptime Feed (Arbitrum One)...");
    let now = chrono::Utc::now().timestamp() as u64;
    let seq_res = check_sequencer(42161, &arb_rpc, now).await;
    println!("  -> Sequencer Issue  : {:?}", seq_res);
    assert_eq!(seq_res, None);

    println!("\n============================================================");
    println!("🏆 ALL 9 LIVE ON-CHAIN REAL-WORLD TESTS PASSED 100%!");
    println!("============================================================");

    Ok(())
}
