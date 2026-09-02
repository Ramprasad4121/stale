---
"@ramprasad4121/stale": minor
---

Introduced two game-changing integration features:

- `createGuardPipeline()`: A composable pre-flight pipeline that chains multiple guardrails into a single `run()` call. Supports fail-fast and run-all modes, async guards, per-guard timing, and fluent chaining. This is the recommended way to integrate stale into production agents.
- `AuditLogger`: A structured compliance-grade audit logger that records every ALLOW/BLOCK decision with timestamps, reasons, and metadata. Supports FIFO eviction, filtering, JSON export, and external callbacks (e.g. ship to Datadog/Splunk).
