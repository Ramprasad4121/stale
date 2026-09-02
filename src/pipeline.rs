use crate::audit::AuditLogger;
use crate::types::{Decision, GuardrailResult};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use std::time::Instant;

pub type BoxedGuardFuture = Pin<Box<dyn Future<Output = GuardrailResult> + Send>>;
pub type GuardFn = Box<dyn Fn() -> BoxedGuardFuture + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineMode {
    FailFast,
    RunAll,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GuardExecutionReport {
    pub name: String,
    pub decision: Decision,
    pub reason: String,
    pub duration_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineResult {
    pub decision: Decision,
    pub reason: String,
    pub guards_run: usize,
    pub guards_passed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_by: Option<String>,
    pub duration_ms: f64,
    pub results: Vec<GuardExecutionReport>,
}

pub struct GuardPipeline {
    guards: Vec<(String, GuardFn)>,
    mode: PipelineMode,
    audit: Option<AuditLogger>,
}

impl GuardPipeline {
    pub fn new(mode: PipelineMode, audit: Option<AuditLogger>) -> Self {
        Self {
            guards: Vec::new(),
            mode,
            audit,
        }
    }

    pub fn add<F, Fut>(&mut self, name: impl Into<String>, f: F) -> &mut Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = GuardrailResult> + Send + 'static,
    {
        let boxed_f: GuardFn = Box::new(move || Box::pin(f()));
        self.guards.push((name.into(), boxed_f));
        self
    }

    pub async fn run(&mut self) -> PipelineResult {
        let start = Instant::now();
        let mut reports = Vec::new();
        let mut blocked = false;
        let mut blocked_by = None;
        let mut block_reason = String::new();

        for (name, guard_fn) in &self.guards {
            let guard_start = Instant::now();
            let result = guard_fn().await;
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
        }
    }
}

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
}
