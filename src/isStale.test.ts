import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { isStale } from "./isStale.js";

describe("isStale", () => {
  const now = 1_724_520_012; // fixed now for deterministic tests

  it("1. fresh: age <= maxAge → ALLOW", () => {
    const r = isStale({ updatedAt: now - 10, nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(r.decision, "ALLOW");
    assert.equal(r.ageSeconds, 10);
    assert.match(r.reason, /ALLOW/);
  });

  it("2. stale: age > maxAge → BLOCK", () => {
    const r = isStale({ updatedAt: now - 100, nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.ageSeconds, 100);
    assert.match(r.reason, /stale/);
  });

  it("3. future updatedAt → BLOCK (not-yet-valid)", () => {
    const r = isStale({ updatedAt: now + 10, nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.ageSeconds, -10);
    assert.match(r.reason, /not-yet-valid/);
  });

  it("4. missing/null/\"\" updatedAt → BLOCK (fail closed)", () => {
    for (const v of [null, undefined, ""]) {
      const r = isStale({ updatedAt: v as any, nowSeconds: now, maxAgeSeconds: 60 });
      assert.equal(r.decision, "BLOCK", `expected BLOCK for ${JSON.stringify(v)}`);
      assert.equal(r.ageSeconds, null);
      assert.match(r.reason, /missing or unparseable/);
    }
    // also "" with spaces
    const r2 = isStale({ updatedAt: "   ", nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(r2.decision, "BLOCK");
    assert.equal(r2.ageSeconds, null);
  });

  it("5. maxAge 0 and age 0 → ALLOW", () => {
    const r = isStale({ updatedAt: now, nowSeconds: now, maxAgeSeconds: 0 });
    assert.equal(r.decision, "ALLOW");
    assert.equal(r.ageSeconds, 0);
  });

  it("6. maxAge 0 and age 1 → BLOCK", () => {
    const r = isStale({ updatedAt: now - 1, nowSeconds: now, maxAgeSeconds: 0 });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.ageSeconds, 1);
  });

  it("7. invalid now or maxAge → BLOCK", () => {
    const badNow = [NaN, Infinity, -Infinity] as const;
    for (const n of badNow) {
      const r = isStale({ updatedAt: now, nowSeconds: n as any, maxAgeSeconds: 60 });
      assert.equal(r.decision, "BLOCK");
      assert.equal(r.ageSeconds, null);
    }
    const badMax = [NaN, Infinity, -Infinity, -1] as const;
    for (const m of badMax) {
      const r = isStale({ updatedAt: now, nowSeconds: now, maxAgeSeconds: m as any });
      assert.equal(r.decision, "BLOCK");
      assert.equal(r.ageSeconds, null);
    }
    // also non-finite string for maxAge handled via Number conversion in CLI, but isStale receives number; test direct NaN
    const r2 = isStale({ updatedAt: now, nowSeconds: now, maxAgeSeconds: NaN });
    assert.equal(r2.decision, "BLOCK");
  });

  it("8. fractional maxAge → BLOCK", () => {
    const r = isStale({ updatedAt: now, nowSeconds: now, maxAgeSeconds: 0.5 });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.ageSeconds, null);
  });

  it("8. bigint updatedAt works", () => {
    const rFresh = isStale({ updatedAt: BigInt(now - 5), nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(rFresh.decision, "ALLOW");
    assert.equal(rFresh.ageSeconds, 5);

    const rStale = isStale({ updatedAt: BigInt(now - 100), nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(rStale.decision, "BLOCK");
    assert.equal(rStale.ageSeconds, 100);

    const rFuture = isStale({ updatedAt: BigInt(now + 5), nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(rFuture.decision, "BLOCK");
    assert.equal(rFuture.ageSeconds, -5);

    // hex string also works via isStale string path
    const rHex = isStale({ updatedAt: `0x${BigInt(now - 5).toString(16)}`, nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(rHex.decision, "ALLOW");
    assert.equal(rHex.ageSeconds, 5);
  });

  it("9. huge and edge values — fail closed on huge future, huge stale", () => {
    // huge future timestamp (2**40) → BLOCK as not-yet-valid
    const hugeFuture = BigInt(1) << BigInt(40); // 1099511627776
    const rFuture = isStale({ updatedAt: hugeFuture, nowSeconds: now, maxAgeSeconds: 60 });
    assert.equal(rFuture.decision, "BLOCK");
    assert.equal(rFuture.ageSeconds! < 0, true);

    // huge stale age (now far in future) → BLOCK
    const rHugeStale = isStale({ updatedAt: now - 1, nowSeconds: Number.MAX_SAFE_INTEGER, maxAgeSeconds: 60 });
    assert.equal(rHugeStale.decision, "BLOCK");
    assert.equal(rHugeStale.ageSeconds! > 60, true);

    // Infinity and NaN for updatedAt string → BLOCK
    for (const v of ["Infinity", "NaN", "1e309"]) {
      const r = isStale({ updatedAt: v, nowSeconds: now, maxAgeSeconds: 60 });
      assert.equal(r.decision, "BLOCK");
      assert.equal(r.ageSeconds, null);
    }
  });
});
