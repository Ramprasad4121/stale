/**
 * @module pipeline
 * Composable Guard Pipeline — the integration layer for stale.
 *
 * Instead of calling 10+ individual guardrails manually, developers define a
 * pipeline of guards and run them all with a single `preflight()` call.
 * The pipeline short-circuits on the first BLOCK (fail-fast), or runs all
 * guards and returns a comprehensive report.
 *
 * Usage:
 *   const pipeline = createGuardPipeline({ mode: "fail-fast", audit: logger });
 *   pipeline.add("priceCheck", () => checkPrice({ ... }));
 *   pipeline.add("gasCheck", () => checkGasPrice({ ... }));
 *   const result = await pipeline.run();
 *   if (result.decision === "BLOCK") throw new Error(result.reason);
 */

import { AuditLogger } from "./audit.js";

export type GuardResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
};

export type GuardFn = () => GuardResult | Promise<GuardResult>;

export type PipelineResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  /** Total number of guards that ran */
  guardsRun: number;
  /** Total number of guards that passed */
  guardsPassed: number;
  /** Name of the guard that blocked (if any) */
  blockedBy?: string;
  /** Duration in milliseconds */
  durationMs: number;
  /** Individual results per guard */
  results: Array<{ name: string; decision: "ALLOW" | "BLOCK"; reason: string; durationMs: number }>;
};

export type PipelineConfig = {
  /**
   * "fail-fast" — stop on first BLOCK (default, recommended for production).
   * "run-all" — run every guard even if one blocks (useful for diagnostics).
   */
  mode?: "fail-fast" | "run-all";
  /** Optional audit logger to record every decision */
  audit?: AuditLogger;
};

export class GuardPipeline {
  private readonly guards: Array<{ name: string; fn: GuardFn }> = [];
  private readonly mode: "fail-fast" | "run-all";
  private readonly audit?: AuditLogger;

  constructor(config: PipelineConfig = {}) {
    this.mode = config.mode ?? "fail-fast";
    this.audit = config.audit;
  }

  /**
   * Add a named guard to the pipeline.
   * Guards run in the order they are added.
   */
  add(name: string, fn: GuardFn): this {
    this.guards.push({ name, fn });
    return this;
  }

  /**
   * Execute the full pre-flight pipeline.
   */
  async run(): Promise<PipelineResult> {
    const start = performance.now();
    const results: PipelineResult["results"] = [];
    let blocked = false;
    let blockedBy: string | undefined;
    let blockReason = "";

    for (const guard of this.guards) {
      const guardStart = performance.now();
      let result: GuardResult;

      try {
        result = await guard.fn();
      } catch (err) {
        // Any guard that throws is a BLOCK (fail closed)
        const msg = err instanceof Error ? err.message : String(err);
        result = { decision: "BLOCK", reason: `guard ${guard.name} threw: ${msg}` };
      }

      const guardDuration = performance.now() - guardStart;

      results.push({
        name: guard.name,
        decision: result.decision,
        reason: result.reason,
        durationMs: Math.round(guardDuration * 100) / 100,
      });

      if (this.audit) {
        this.audit.record(guard.name, result.decision, result.reason);
      }

      if (result.decision === "BLOCK") {
        blocked = true;
        if (!blockedBy) {
          blockedBy = guard.name;
          blockReason = result.reason;
        }

        if (this.mode === "fail-fast") {
          break;
        }
      }
    }

    const totalDuration = performance.now() - start;

    return {
      decision: blocked ? "BLOCK" : "ALLOW",
      reason: blocked ? blockReason : "all guards passed",
      guardsRun: results.length,
      guardsPassed: results.filter((r) => r.decision === "ALLOW").length,
      blockedBy,
      durationMs: Math.round(totalDuration * 100) / 100,
      results,
    };
  }
}

/**
 * Factory function to create a new guard pipeline.
 */
export function createGuardPipeline(config?: PipelineConfig): GuardPipeline {
  return new GuardPipeline(config);
}
