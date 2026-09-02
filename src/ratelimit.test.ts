import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { RateLimiter, SpendingCap } from "./ratelimit.js";

describe("RateLimiter", () => {
  it("ALLOWs under limit", () => {
    const rl = new RateLimiter({ maxTransactions: 3, windowSeconds: 60 });
    assert.equal(rl.check().decision, "ALLOW");
    rl.record();
    assert.equal(rl.check().decision, "ALLOW");
    rl.record();
    assert.equal(rl.check().decision, "ALLOW");
  });

  it("BLOCKs when limit exceeded", () => {
    const rl = new RateLimiter({ maxTransactions: 2, windowSeconds: 60 });
    rl.record();
    rl.record();
    const res = rl.check();
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /RATE LIMIT EXCEEDED/);
  });

  it("remaining() reports correctly", () => {
    const rl = new RateLimiter({ maxTransactions: 5, windowSeconds: 60 });
    assert.equal(rl.remaining(), 5);
    rl.record();
    rl.record();
    assert.equal(rl.remaining(), 3);
  });

  it("throws on invalid config", () => {
    assert.throws(() => new RateLimiter({ maxTransactions: 0, windowSeconds: 60 }));
    assert.throws(() => new RateLimiter({ maxTransactions: 5, windowSeconds: -1 }));
  });
});

describe("SpendingCap", () => {
  it("ALLOWs under cap", () => {
    const sc = new SpendingCap({ maxSpend: 1000n, windowSeconds: 60 });
    assert.equal(sc.check(500n).decision, "ALLOW");
    sc.record(500n);
    assert.equal(sc.check(400n).decision, "ALLOW");
  });

  it("BLOCKs when cap exceeded", () => {
    const sc = new SpendingCap({ maxSpend: 1000n, windowSeconds: 60 });
    sc.record(800n);
    const res = sc.check(300n);
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /SPENDING CAP EXCEEDED/);
  });

  it("remaining() reports correctly", () => {
    const sc = new SpendingCap({ maxSpend: 1000n, windowSeconds: 60 });
    assert.equal(sc.remaining(), 1000n);
    sc.record(300n);
    assert.equal(sc.remaining(), 700n);
  });

  it("BLOCKs on invalid proposedAmount", () => {
    const sc = new SpendingCap({ maxSpend: 1000n, windowSeconds: 60 });
    assert.equal(sc.check(-1n).decision, "BLOCK");
    // @ts-expect-error
    assert.equal(sc.check(100).decision, "BLOCK");
  });

  it("throws on invalid config", () => {
    assert.throws(() => new SpendingCap({ maxSpend: 0n, windowSeconds: 60 }));
    assert.throws(() => new SpendingCap({ maxSpend: 1000n, windowSeconds: -1 }));
  });
});
