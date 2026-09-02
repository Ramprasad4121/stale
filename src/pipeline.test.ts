import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { createGuardPipeline } from "./pipeline.js";
import { AuditLogger } from "./audit.js";

describe("GuardPipeline", () => {
  it("ALLOWs when all guards pass", async () => {
    const pipeline = createGuardPipeline();
    pipeline.add("a", () => ({ decision: "ALLOW", reason: "ok" }));
    pipeline.add("b", () => ({ decision: "ALLOW", reason: "fine" }));

    const result = await pipeline.run();
    assert.equal(result.decision, "ALLOW");
    assert.equal(result.guardsRun, 2);
    assert.equal(result.guardsPassed, 2);
    assert.equal(result.blockedBy, undefined);
  });

  it("BLOCKs on first failure in fail-fast mode", async () => {
    const pipeline = createGuardPipeline({ mode: "fail-fast" });
    pipeline.add("a", () => ({ decision: "ALLOW", reason: "ok" }));
    pipeline.add("b", () => ({ decision: "BLOCK", reason: "gas too high" }));
    pipeline.add("c", () => ({ decision: "ALLOW", reason: "ok" })); // should not run

    const result = await pipeline.run();
    assert.equal(result.decision, "BLOCK");
    assert.equal(result.guardsRun, 2); // 'c' never ran
    assert.equal(result.guardsPassed, 1);
    assert.equal(result.blockedBy, "b");
    assert.match(result.reason, /gas too high/);
  });

  it("runs all guards in run-all mode", async () => {
    const pipeline = createGuardPipeline({ mode: "run-all" });
    pipeline.add("a", () => ({ decision: "ALLOW", reason: "ok" }));
    pipeline.add("b", () => ({ decision: "BLOCK", reason: "blocked" }));
    pipeline.add("c", () => ({ decision: "ALLOW", reason: "ok" }));

    const result = await pipeline.run();
    assert.equal(result.decision, "BLOCK");
    assert.equal(result.guardsRun, 3); // all ran
    assert.equal(result.guardsPassed, 2);
    assert.equal(result.blockedBy, "b");
  });

  it("handles async guards", async () => {
    const pipeline = createGuardPipeline();
    pipeline.add("async-guard", async () => {
      await new Promise((r) => setTimeout(r, 5));
      return { decision: "ALLOW" as const, reason: "async ok" };
    });

    const result = await pipeline.run();
    assert.equal(result.decision, "ALLOW");
    assert.ok(result.durationMs >= 0);
  });

  it("BLOCKs (fail closed) if a guard throws", async () => {
    const pipeline = createGuardPipeline();
    pipeline.add("thrower", () => {
      throw new Error("rpc exploded");
    });

    const result = await pipeline.run();
    assert.equal(result.decision, "BLOCK");
    assert.match(result.reason, /rpc exploded/);
    assert.equal(result.blockedBy, "thrower");
  });

  it("integrates with AuditLogger", async () => {
    const logger = new AuditLogger();
    const pipeline = createGuardPipeline({ audit: logger });
    pipeline.add("price", () => ({ decision: "ALLOW", reason: "fresh" }));
    pipeline.add("gas", () => ({ decision: "BLOCK", reason: "too high" }));

    await pipeline.run();

    assert.equal(logger.size(), 2);
    assert.equal(logger.getBlocks().length, 1);
    assert.equal(logger.getByGuardrail("price")[0].decision, "ALLOW");
  });

  it("reports per-guard timing", async () => {
    const pipeline = createGuardPipeline();
    pipeline.add("fast", () => ({ decision: "ALLOW", reason: "ok" }));

    const result = await pipeline.run();
    assert.equal(result.results.length, 1);
    assert.equal(result.results[0].name, "fast");
    assert.ok(typeof result.results[0].durationMs === "number");
  });

  it("supports fluent chaining", async () => {
    const result = await createGuardPipeline()
      .add("a", () => ({ decision: "ALLOW", reason: "ok" }))
      .add("b", () => ({ decision: "ALLOW", reason: "ok" }))
      .run();

    assert.equal(result.decision, "ALLOW");
    assert.equal(result.guardsRun, 2);
  });
});
