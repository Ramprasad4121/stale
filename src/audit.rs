use crate::types::Decision;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditEntry {
    pub timestamp: String,
    pub guardrail: String,
    pub decision: Decision,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

pub type OnEntryCallback = Arc<dyn Fn(&AuditEntry) + Send + Sync>;

#[derive(Clone)]
pub struct AuditLogger {
    entries: Vec<AuditEntry>,
    max_entries: usize,
    on_entry: Option<OnEntryCallback>,
}

impl AuditLogger {
    pub fn new(max_entries: Option<usize>, on_entry: Option<OnEntryCallback>) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: max_entries.unwrap_or(10_000),
            on_entry,
        }
    }

    pub fn record(
        &mut self,
        guardrail: impl Into<String>,
        decision: Decision,
        reason: impl Into<String>,
        metadata: Option<serde_json::Value>,
    ) -> AuditEntry {
        let entry = AuditEntry {
            timestamp: Utc::now().to_rfc3339(),
            guardrail: guardrail.into(),
            decision,
            reason: reason.into(),
            metadata,
        };

        self.entries.push(entry.clone());

        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        if let Some(ref cb) = self.on_entry {
            cb(&entry);
        }

        entry
    }

    pub fn get_entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    pub fn get_blocks(&self) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.decision == Decision::Block)
            .collect()
    }

    pub fn get_by_guardrail(&self, name: &str) -> Vec<&AuditEntry> {
        self.entries
            .iter()
            .filter(|e| e.guardrail == name)
            .collect()
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(&self.entries)
    }
}

impl Default for AuditLogger {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audit_logger_records_and_filters() {
        let mut logger = AuditLogger::default();
        logger.record("gas", Decision::Allow, "gas price ok", None);
        logger.record("honeypot", Decision::Block, "transfer reverted", None);

        assert_eq!(logger.size(), 2);
        assert_eq!(logger.get_blocks().len(), 1);
        assert_eq!(logger.get_blocks()[0].guardrail, "honeypot");
        assert_eq!(logger.get_by_guardrail("gas").len(), 1);
    }
}
