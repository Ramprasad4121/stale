import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { quoteFromFeed } from "./quote.js";

describe("quoteFromFeed", () => {
  it("245377000000n / 8 → 2453.77", () => {
    const { priceUsd, quoteUsd } = quoteFromFeed({
      answer: 245377000000n,
      decimals: 8,
      amountEth: null,
    });
    assert.equal(priceUsd, 2453.77);
    assert.equal(quoteUsd, null);
  });

  it("*0.5 → 1226.885", () => {
    const { priceUsd, quoteUsd } = quoteFromFeed({
      answer: 245377000000n,
      decimals: 8,
      amountEth: 0.5,
    });
    assert.equal(priceUsd, 2453.77);
    assert.equal(quoteUsd, 1226.885);
  });

  it("null amount → null quote", () => {
    const { quoteUsd } = quoteFromFeed({ answer: 100000000n, decimals: 8, amountEth: null });
    assert.equal(quoteUsd, null);
    const { quoteUsd: q2 } = quoteFromFeed({ answer: 100000000n, decimals: 8 });
    assert.equal(q2, null);
  });

  it("0n → throw (fail closed)", () => {
    assert.throws(
      () => quoteFromFeed({ answer: 0n, decimals: 8, amountEth: null }),
      /invalid answer/,
    );
    assert.throws(
      () => quoteFromFeed({ answer: -1n, decimals: 8, amountEth: null }),
      /invalid answer/,
    );
  });

  it("bad decimals → throw", () => {
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: -1, amountEth: null }),
      /invalid decimals/,
    );
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: 37, amountEth: null }),
      /invalid decimals/,
    );
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: 8.5, amountEth: null }),
      /invalid decimals/,
    );
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: NaN, amountEth: null }),
      /invalid decimals/,
    );
  });

  it("NaN amount → throw", () => {
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: 8, amountEth: NaN }),
      /invalid amountEth/,
    );
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: 8, amountEth: Infinity }),
      /invalid amountEth/,
    );
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: 8, amountEth: -1 }),
      /invalid amountEth/,
    );
  });

  it("huge answer and overflow — handled", () => {
    const huge = BigInt(
      "115792089237316195423570985008687907853269984665640564039457584007913129639935",
    ); // 2**256-1
    // With 8 decimals, huge maps to ~1e69, still finite in JS (1e69 < 1e308) — should not throw, just large
    const rHuge = quoteFromFeed({ answer: huge, decimals: 8, amountEth: null });
    assert.equal(Number.isFinite(rHuge.priceUsd), true);
    // huge amountEth * priceUsd overflow → Infinity → throw invalid quoteUsd
    assert.throws(
      () => quoteFromFeed({ answer: 100000000000n, decimals: 8, amountEth: Number.MAX_VALUE }),
      /invalid quoteUsd/,
    );
    // Infinity decimals → throw
    assert.throws(
      () => quoteFromFeed({ answer: 100n, decimals: Infinity, amountEth: null }),
      /invalid decimals/,
    );
  });
});
