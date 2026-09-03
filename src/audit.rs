//! Compliance-grade audit trail for every ALLOW/BLOCK decision.
//!
//! [`AuditLogger`] is an in-memory bounded FIFO. It is a *record* of guard
//! outcomes — it never gates execution itself. The `on_entry` callback is
//! invoked synchronously after each record; panics in the callback are
//! caught so logging can never break a guard, but slow callbacks DO slow
//! the pipeline — keep them non-blocking (e.g. channel send).

use crate::types::Decision;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

/// Default bound when `max_entries` is `None`.
pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// One recorded guard outcome.
pub struct AuditEntry {
    pub timestamp: String,
    pub guardrail: String,
    pub decision: Decision,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Sync callback invoked per entry. Keep non-blocking; panics are caught.
pub type OnEntryCallback = Arc<dyn Fn(&AuditEntry) + Send + Sync>;

#[derive(Clone)]
/// Bounded FIFO audit log (`VecDeque`, O(1) eviction).
pub struct AuditLogger {
    entries: VecDeque<AuditEntry>,
    max_entries: usize,
    on_entry: Option<OnEntryCallback>,
}

impl AuditLogger {
    /// Create a logger. `max_entries`: `None` → [`DEFAULT_MAX_ENTRIES`];
    /// `Some(0)` is coerced to `1` (a zero-capacity log would silently drop
    /// all history — fail loud by keeping at least one entry).
    pub fn new(max_entries: Option<usize>, on_entry: Option<OnEntryCallback>) -> Self {
        let max = match max_entries {
            None => DEFAULT_MAX_ENTRIES,
            Some(0) => 1,
            Some(n) => n,
        };
        Self {
            entries: VecDeque::new(),
            max_entries: max,
            on_entry,
        }
    }

    /// Record one guard outcome, evicting oldest-first past capacity.
    /// Callback panics are swallowed (logged path must not break guards).
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

        self.entries.push_back(entry.clone());

        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }

        if let Some(ref cb) = self.on_entry {
            // Panic isolation: audit must never break the guard path.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cb(&entry)));
        }

        entry
    }

    /// All entries, oldest first.
    pub fn get_entries(&self) -> Vec<&AuditEntry> {
        self.entries.iter().collect()
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

    #[test]
    fn test_zero_capacity_coerced_to_one() {
        let mut logger = AuditLogger::new(Some(0), None);
        logger.record("a", Decision::Allow, "x", None);
        logger.record("b", Decision::Allow, "y", None);
        // Capacity 1: only the newest entry survives, nothing silently unbounded.
        assert_eq!(logger.size(), 1);
        assert_eq!(logger.get_entries()[0].guardrail, "b");
    }

    #[test]
    fn test_callback_panic_does_not_break_record() {
        let cb: OnEntryCallback = Arc::new(|_| panic!("logger boom"));
        let mut logger = AuditLogger::new(None, Some(cb));
        let entry = logger.record("gas", Decision::Allow, "ok", None);
        assert_eq!(entry.guardrail, "gas");
        assert_eq!(logger.size(), 1);
    }
}
