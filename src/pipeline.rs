//! Composable fail-closed pre-flight pipeline.
//!
//! Chains ordered guardrails into a single [`GuardPipeline::run`] call.
//! Every guard returns [`GuardrailResult`]; any `Block` poisons the final
//! decision. Guards run sequentially in registration order.
//!
//! # Liveness
//! Each guard is bounded by a per-guard timeout
//! ([`GuardPipeline::with_guard_timeout`], default
//! [`DEFAULT_GUARD_TIMEOUT`]). A hung `eth_call` becomes a `Block`, never a
//! stall. Guard panics are caught and converted to `Block` for the same
//! reason — one faulty guard must not abort the whole preflight.

use crate::audit::AuditLogger;
use crate::types::{Decision, GuardrailResult};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::{Duration, Instant};

/// Default per-guard timeout: 15s. Covers the 10s RPC timeout plus margin.
pub const DEFAULT_GUARD_TIMEOUT: Duration = Duration::from_secs(15);
/// Hard cap on guards per pipeline (misconfiguration / DoS bound).
pub const MAX_GUARDS: usize = 64;

/// Boxed guard future. `'static` is required so `run()` can `tokio::spawn`
/// each guard for panic isolation.
pub type BoxedGuardFuture = Pin<Box<dyn Future<Output = GuardrailResult> + Send + 'static>>;
pub type GuardFn = Box<dyn Fn() -> BoxedGuardFuture + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// `FailFast` stops at the first `Block`; `RunAll` executes every guard
/// (useful for audit trails showing all violations at once).
pub enum PipelineMode {
    FailFast,
    RunAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Per-guard timing + outcome. `duration_ms` is wall-clock including timeout.
pub struct GuardExecutionReport {
    pub name: String,
    pub decision: Decision,
    pub reason: String,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// Aggregate preflight verdict. `decision` is `Block` if ANY guard blocked,
/// timed out, or panicked. `blocked_by` names the first blocking guard.
/// `guards_skipped` names guards never executed because `FailFast` stopped
/// early — the audit trail is explicit about what did NOT run.
pub struct PipelineResult {
    pub decision: Decision,
    pub reason: String,
    pub guards_run: usize,
    pub guards_passed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<String>,
    pub duration_ms: f64,
    pub results: Vec<GuardExecutionReport>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub guards_skipped: Vec<String>,
}

/// Ordered, fail-closed guard runner. See module docs for liveness guarantees.
pub struct GuardPipeline {
    guards: Vec<(String, GuardFn)>,
    mode: PipelineMode,
    audit: Option<AuditLogger>,
    guard_timeout: Duration,
}

impl GuardPipeline {
    /// Create a pipeline. `audit` receives one entry per executed guard.
    pub fn new(mode: PipelineMode, audit: Option<AuditLogger>) -> Self {
        Self {
            guards: Vec::new(),
            mode,
            audit,
            guard_timeout: DEFAULT_GUARD_TIMEOUT,
        }
    }

    /// Override the per-guard timeout. Values below 1ms are clamped to 1ms;
    /// a zero timeout would fail-closed on every guard.
    pub fn with_guard_timeout(mut self, timeout: Duration) -> Self {
        self.guard_timeout = timeout.max(Duration::from_millis(1));
        self
    }

    /// Register a guard. Panics if more than [`MAX_GUARDS`] are added —
    /// a pipeline that large is a misconfiguration; fail loud at build
    /// time rather than timing out at preflight time.
    pub fn add<F, Fut>(&mut self, name: impl Into<String>, f: F) -> &mut Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = GuardrailResult> + Send + 'static,
    {
        let name = name.into();
        assert!(
            self.guards.len() < MAX_GUARDS,
            "GuardPipeline exceeds MAX_GUARDS ({}): refusing to add '{}'",
            MAX_GUARDS,
            name
        );
        let boxed_f: GuardFn = Box::new(move || Box::pin(f()));
        self.guards.push((name, boxed_f));
        self
    }

    /// Run guards sequentially in registration order.
    ///
    /// Each guard is wrapped in [`tokio::time::timeout`] (see
    /// `guard_timeout`) and panic isolation: timeouts and panics yield a
    /// `Block` attributed to that guard instead of stalling or aborting
    /// the preflight.
    pub async fn run(&mut self) -> PipelineResult {
        let start = Instant::now();
        let mut reports = Vec::new();
        let mut blocked = false;
        let mut blocked_by = None;
        let mut block_reason = String::new();
        let mut skipped: Vec<String> = Vec::new();

        for (index, (name, guard_fn)) in self.guards.iter().enumerate() {
            let guard_start = Instant::now();
            // Spawn per guard: a panicking guard surfaces as JoinError
            // (converted to BLOCK below) instead of aborting preflight.
            // Timeout bounds hung RPCs.
            let result =
                match tokio::time::timeout(self.guard_timeout, tokio::spawn(guard_fn())).await {
                    Ok(Ok(res)) => res,
                    Ok(Err(join_err)) => {
                        if join_err.is_panic() {
                            GuardrailResult::block(format!(
                                "guard '{}' panicked — BLOCK (fail closed)",
                                name
                            ))
                        } else {
                            GuardrailResult::block(format!(
                                "guard '{}' task failed (cancelled) — BLOCK (fail closed)",
                                name
                            ))
                        }
                    }
                    Err(_) => GuardrailResult::block(format!(
                        "guard '{}' timed out after {:?} — BLOCK (fail closed)",
                        name, self.guard_timeout
                    )),
                };
            let guard_duration = guard_start.elapsed().as_secs_f64() * 1000.0;

            reports.push(GuardExecutionReport {
                name: name.clone(),
                decision: result.decision,
                reason: result.reason.clone(),
                duration_ms: (guard_duration * 100.0).round() / 100.0,
            });

            if let Some(ref mut audit) = self.audit {
                audit.record(name, result.decision, &result.reason, result.metadata);
            }

            if result.decision == Decision::Block {
                blocked = true;
                if blocked_by.is_none() {
                    blocked_by = Some(name.clone());
                    block_reason = result.reason;
                }

                if self.mode == PipelineMode::FailFast {
                    // Name every guard that will NOT run so the audit
                    // trail never silently omits unevaluated guards.
                    skipped.extend(self.guards[index + 1..].iter().map(|(n, _)| n.clone()));
                    break;
                }
            }
        }

        let total_duration = start.elapsed().as_secs_f64() * 1000.0;
        let guards_passed = reports
            .iter()
            .filter(|r| r.decision == Decision::Allow)
            .count();

        PipelineResult {
            decision: if blocked {
                Decision::Block
            } else {
                Decision::Allow
            },
            reason: if blocked {
                block_reason
            } else {
                "all guards passed".to_string()
            },
            guards_run: reports.len(),
            guards_passed,
            blocked_by,
            duration_ms: (total_duration * 100.0).round() / 100.0,
            results: reports,
            guards_skipped: skipped,
        }
    }
}

/// Build an empty pipeline in `mode` with optional audit sink.
pub fn create_guard_pipeline(mode: PipelineMode, audit: Option<AuditLogger>) -> GuardPipeline {
    GuardPipeline::new(mode, audit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_pipeline_all_pass() {
        let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);
        pipeline.add("guard1", || async { GuardrailResult::allow("ok 1") });
        pipeline.add("guard2", || async { GuardrailResult::allow("ok 2") });

        let res = pipeline.run().await;
        assert_eq!(res.decision, Decision::Allow);
        assert_eq!(res.guards_run, 2);
        assert_eq!(res.guards_passed, 2);
    }

    #[tokio::test]
    async fn test_pipeline_fail_fast_stops_at_first_block() {
        let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);
        pipeline.add("guard1", || async { GuardrailResult::allow("ok 1") });
        pipeline.add("guard2", || async { GuardrailResult::block("failed at 2") });
        pipeline.add("guard3", || async { GuardrailResult::allow("ok 3") });

        let res = pipeline.run().await;
        assert_eq!(res.decision, Decision::Block);
        assert_eq!(res.guards_run, 2); // stopped at guard2
        assert_eq!(res.blocked_by, Some("guard2".to_string()));
    }

    #[tokio::test]
    async fn test_pipeline_fail_fast_names_skipped_guards() {
        let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);
        pipeline.add("guard1", || async { GuardrailResult::allow("ok 1") });
        pipeline.add("guard2", || async { GuardrailResult::block("failed at 2") });
        pipeline.add("guard3", || async { GuardrailResult::allow("ok 3") });

        let res = pipeline.run().await;
        assert_eq!(res.guards_skipped, vec!["guard3".to_string()]);

        // RunAll never skips: full trail, empty skipped list.
        let mut full = create_guard_pipeline(PipelineMode::RunAll, None);
        full.add("guard1", || async { GuardrailResult::allow("ok 1") });
        full.add("guard2", || async { GuardrailResult::block("failed at 2") });
        full.add("guard3", || async { GuardrailResult::allow("ok 3") });

        let res = full.run().await;
        assert!(res.guards_skipped.is_empty());
    }

    #[tokio::test]
    async fn test_pipeline_run_all_runs_all_guards() {
        let mut pipeline = create_guard_pipeline(PipelineMode::RunAll, None);
        pipeline.add("guard1", || async { GuardrailResult::allow("ok 1") });
        pipeline.add("guard2", || async { GuardrailResult::block("failed at 2") });
        pipeline.add("guard3", || async { GuardrailResult::allow("ok 3") });

        let res = pipeline.run().await;
        assert_eq!(res.decision, Decision::Block);
        assert_eq!(res.guards_run, 3);
        assert_eq!(res.guards_passed, 2);
    }

    #[tokio::test]
    async fn test_pipeline_guard_timeout_blocks_fail_closed() {
        let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None)
            .with_guard_timeout(std::time::Duration::from_millis(20));
        pipeline.add("hung", || async {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            GuardrailResult::allow("never")
        });

        let res = pipeline.run().await;
        assert_eq!(res.decision, Decision::Block);
        assert_eq!(res.blocked_by, Some("hung".to_string()));
        assert!(res.reason.contains("timed out"));
    }

    #[tokio::test]
    async fn test_pipeline_guard_panic_blocks_fail_closed() {
        let mut pipeline = create_guard_pipeline(PipelineMode::FailFast, None);
        pipeline.add("panicker", || async {
            panic!("boom");
            #[allow(unreachable_code)]
            GuardrailResult::allow("never")
        });

        let res = pipeline.run().await;
        assert_eq!(res.decision, Decision::Block);
        assert_eq!(res.blocked_by, Some("panicker".to_string()));
        assert!(res.reason.contains("panicked"));
    }
}
