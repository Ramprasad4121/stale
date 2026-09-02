use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
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
pub struct GuardrailResult {
    pub decision: Decision,
    pub reason: String,
    pub allow_execute: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

impl GuardrailResult {
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            decision: Decision::Allow,
            reason: reason.into(),
            allow_execute: true,
            metadata: None,
        }
    }

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
}
