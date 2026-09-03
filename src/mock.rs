//! Test-only mock transport.
//!
//! # Fidelity caveat
//! `call_from` falls back to `call_handler` when no `call_from_handler` is
//! set, silently ignoring `from`. Tests for `from`-dependent guards
//! (honeypot, simulation) MUST set an explicit `call_from_handler`.

use crate::rpc::EvmRpcClient;
use async_trait::async_trait;
use std::sync::Arc;

pub type CallHandler = Arc<dyn Fn(&str, &str) -> Result<String, String> + Send + Sync>;
pub type CallFromHandler = Arc<dyn Fn(&str, &str, &str) -> Result<String, String> + Send + Sync>;

#[derive(Default, Clone)]
/// In-memory mock. Every unset field errors on use (fail-closed tests).
pub struct MockRpcClient {
    pub call_handler: Option<CallHandler>,
    pub call_from_handler: Option<CallFromHandler>,
    pub block_number: Option<u64>,
    pub block_timestamp: Option<u64>,
    pub chain_id: Option<u64>,
    pub nonce: Option<u64>,
    pub gas_price: Option<u128>,
    pub base_fee: Option<u128>,
    pub priority_fee: Option<u128>,
    pub balance: Option<u128>,
    pub code: Option<String>,
}

#[async_trait]
impl EvmRpcClient for MockRpcClient {
    async fn call(&self, to: &str, data: &str) -> Result<String, String> {
        if let Some(ref handler) = self.call_handler {
            handler(to, data)
        } else {
            Err("mock call handler not configured".to_string())
        }
    }

    async fn call_from(&self, from: &str, to: &str, data: &str) -> Result<String, String> {
        if let Some(ref handler) = self.call_from_handler {
            handler(from, to, data)
        } else if let Some(ref handler) = self.call_handler {
            handler(to, data)
        } else {
            Err("mock call handler not configured".to_string())
        }
    }

    async fn get_block_number(&self) -> Result<u64, String> {
        self.block_number
            .ok_or_else(|| "mock block_number not configured".to_string())
    }

    async fn get_block_timestamp(&self) -> Result<u64, String> {
        self.block_timestamp
            .ok_or_else(|| "mock block_timestamp not configured".to_string())
    }

    async fn get_chain_id(&self) -> Result<u64, String> {
        self.chain_id
            .ok_or_else(|| "mock chain_id not configured".to_string())
    }

    async fn get_transaction_count(&self, _address: &str) -> Result<u64, String> {
        self.nonce
            .ok_or_else(|| "mock nonce not configured".to_string())
    }

    async fn get_gas_price(&self) -> Result<u128, String> {
        self.gas_price
            .ok_or_else(|| "mock gas_price not configured".to_string())
    }

    async fn get_base_fee(&self) -> Result<u128, String> {
        self.base_fee
            .ok_or_else(|| "mock base_fee not configured".to_string())
    }

    async fn get_priority_fee(&self) -> Result<u128, String> {
        self.priority_fee
            .ok_or_else(|| "mock priority_fee not configured".to_string())
    }

    async fn get_balance(&self, _address: &str) -> Result<u128, String> {
        self.balance
            .ok_or_else(|| "mock balance not configured".to_string())
    }

    async fn get_code(&self, _address: &str) -> Result<String, String> {
        self.code
            .clone()
            .ok_or_else(|| "mock code not configured".to_string())
    }
}
