//! `stale`: fail-closed DeFi guardrails for autonomous AI agents.
//!
//! Before an agent signs and broadcasts, run pre-flight checks against
//! on-chain state and oracle feeds. Any doubt → [`types::Decision::Block`].
//!
//! # Quickstart
//! ```rust,no_run
//! use stale::prelude::*;
//! use stale::types::GuardrailResult;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let rpc = HttpRpcClient::new("https://rpc.flashbots.net");
//!     let mut allowlist = HashMap::new();
//!     allowlist.insert("UNISWAP_V3_ROUTER".into(), "0xE592427A0AEce92De3Edee1F18E0157C05861564".into());
//!     let book = AddressBook::new(allowlist, true)?;
//!     let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);
//!     pipeline.add("address_book", || async { GuardrailResult::allow("ok") });
//!     let report = pipeline.run().await;
//!     assert_eq!(report.decision, Decision::Allow);
//!     Ok(())
//! }
//! ```
//!
//! # Fail-closed contract
//! Every guard returns [`types::GuardrailResult`] with
//! `allow_execute == (decision == Allow)`. The only way to get `ALLOW` is
//! to pass *every* check; RPC failures, decode failures, stale data, and
//! misconfiguration all yield `BLOCK`.

pub mod abi;
pub mod addressbook;
pub mod allowance;
pub mod audit;
pub mod check;
pub mod contract;
pub mod deadline;
pub mod deviation;
pub mod dex;
pub mod feeds;
pub mod gas;
pub mod honeypot;
pub mod is_stale;
pub mod mev;
pub mod mock;
pub mod network;
pub mod pausable;
pub mod pipeline;
pub mod quote;
pub mod ratelimit;
pub mod rpc;
pub mod sanctions;
pub mod sequencer;
pub mod simulate;
pub mod slippage;
pub mod solvency;
pub mod types;

// Re-exports
pub use addressbook::AddressBook;
pub use allowance::{check_allowance, check_approval};
pub use audit::{AuditEntry, AuditLogger};
pub use check::{check_price, check_prices, CheckPriceInput, CheckPriceResult};
pub use contract::check_is_contract;
pub use deadline::{check_deadline, CheckDeadlineInput};
pub use deviation::check_price_deviation;
pub use dex::{check_pool_v2, check_pool_v3};
pub use feeds::{lookup_feed, FeedEntry, DEFAULT_FEED, FEEDS, REGISTRY};
pub use gas::{check_gas_price, check_gas_price_1559};
pub use honeypot::check_token_tax;
pub use is_stale::{is_stale, IsStaleInput, IsStaleResult};
pub use mev::{check_mev_rpc, MEV_PROTECTED_RPCS};
pub use mock::MockRpcClient;
pub use network::{check_chain_id, check_nonce, check_rpc_sync};
pub use pausable::check_paused;
pub use pipeline::{
    create_guard_pipeline, GuardExecutionReport, GuardPipeline, PipelineMode, PipelineResult,
    DEFAULT_GUARD_TIMEOUT, MAX_GUARDS,
};
pub use quote::{quote_from_feed, QuoteInput, QuoteResult};
pub use ratelimit::{RateLimiter, SpendingCap};
pub use rpc::{EvmRpcClient, HttpRpcClient};
pub use sanctions::{check_sanctioned, SANCTIONS_ORACLE};
pub use sequencer::check_sequencer;
pub use simulate::{simulate_tx, SimulateTxInput};
pub use slippage::{calculate_min_amount_out, CalculateMinAmountOutInput};
pub use solvency::check_balance;
pub use types::{Decision, GuardrailResult};

pub mod prelude {
    pub use crate::addressbook::AddressBook;
    pub use crate::allowance::{check_allowance, check_approval};
    pub use crate::audit::{AuditEntry, AuditLogger};
    pub use crate::check::{check_price, check_prices, CheckPriceInput, CheckPriceResult};
    pub use crate::contract::check_is_contract;
    pub use crate::deadline::{check_deadline, CheckDeadlineInput};
    pub use crate::deviation::check_price_deviation;
    pub use crate::dex::{check_pool_v2, check_pool_v3};
    pub use crate::feeds::{lookup_feed, DEFAULT_FEED};
    pub use crate::gas::{check_gas_price, check_gas_price_1559};
    pub use crate::honeypot::check_token_tax;
    pub use crate::is_stale::{is_stale, IsStaleInput};
    pub use crate::mev::check_mev_rpc;
    pub use crate::network::{check_chain_id, check_nonce, check_rpc_sync};
    pub use crate::pausable::check_paused;
    pub use crate::pipeline::{
        create_guard_pipeline, GuardPipeline, PipelineMode, DEFAULT_GUARD_TIMEOUT,
    };
    pub use crate::quote::{quote_from_feed, QuoteInput};
    pub use crate::ratelimit::{RateLimiter, SpendingCap};
    pub use crate::rpc::{EvmRpcClient, HttpRpcClient};
    pub use crate::sanctions::check_sanctioned;
    pub use crate::sequencer::check_sequencer;
    pub use crate::simulate::{simulate_tx, SimulateTxInput};
    pub use crate::slippage::{calculate_min_amount_out, CalculateMinAmountOutInput};
    pub use crate::solvency::check_balance;
    pub use crate::types::{Decision, GuardrailResult};
}

/// Crate version (`CARGO_PKG_VERSION`).
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}
