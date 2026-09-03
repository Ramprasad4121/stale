//! Core verdict types.
//!
//! # Invariant
//! `allow_execute == (decision == Decision::Allow)`. Constructors
//! ([`GuardrailResult::allow`] / [`block`](GuardrailResult::block))
//! uphold it. Deserialized values from untrusted JSON SHOULD be
//! re-checked with `GuardrailResult::is_blocked` / `is_allowed`
//! rather than trusting a possibly inconsistent `allow_execute` flag.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
/// Final verdict. There is no third state: doubt → `Block`.
pub enum Decision {
    Allow,
    Block,
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Allow => write!(f, "ALLOW"),
            Decision::Block => write!(f, "BLOCK"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Guard outcome. Prefer [`is_allowed`](Self::is_allowed) /
/// [`is_blocked`](Self::is_blocked) (derived from `decision`) over reading
/// `allow_execute` on values deserialized from untrusted sources.
pub struct GuardrailResult {
    pub decision: Decision,
    pub reason: String,
    pub allow_execute: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl GuardrailResult {
    /// Construct an `Allow` (sets `allow_execute: true`).
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            decision: Decision::Allow,
            reason: reason.into(),
            allow_execute: true,
            metadata: None,
        }
    }

    /// Construct a `Block` (sets `allow_execute: false`).
    pub fn block(reason: impl Into<String>) -> Self {
        Self {
            decision: Decision::Block,
            reason: reason.into(),
            allow_execute: false,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// True iff `decision == Allow` (ignores the serialized flag).
    pub fn is_allowed(&self) -> bool {
        self.decision == Decision::Allow
    }

    /// True iff `decision == Block` (ignores the serialized flag).
    pub fn is_blocked(&self) -> bool {
        self.decision == Decision::Block
    }
}
