import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { calculateMinAmountOut } from "./slippage.js";

describe("calculateMinAmountOut", () => {
  it("calculates exact amount out with 0 slippage (WETH -> USDC)", () => {
    // 1 WETH (18 dec) @ $3000 (8 dec) -> USDC (6 dec) @ $1 (8 dec)
    const minOut = calculateMinAmountOut({
      amountIn: 1000000000000000000n, // 1 WETH
      tokenInDecimals: 18,
      priceInAnswer: 300000000000n, // $3000
      priceInDecimals: 8,
      tokenOutDecimals: 6,
      priceOutAnswer: 100000000n, // $1
      priceOutDecimals: 8,
      slippageBps: 0,
    });
    // Should be 3000 USDC -> 3000 * 10^6 = 3000000000
    assert.equal(minOut, 3000000000n);
  });

  it("calculates with 50 bps (0.5%) slippage", () => {
    const minOut = calculateMinAmountOut({
      amountIn: 1000000000000000000n, // 1 WETH
      tokenInDecimals: 18,
      priceInAnswer: 300000000000n, // $3000
      priceInDecimals: 8,
      tokenOutDecimals: 6,
      priceOutAnswer: 100000000n, // $1
      priceOutDecimals: 8,
      slippageBps: 50,
    });
    // 3000 * 0.995 = 2985 USDC -> 2985000000
    assert.equal(minOut, 2985000000n);
  });

  it("handles different decimals natively (USDC -> WETH)", () => {
    const minOut = calculateMinAmountOut({
      amountIn: 3000000000n, // 3000 USDC
      tokenInDecimals: 6,
      priceInAnswer: 100000000n, // $1
      priceInDecimals: 8,
      tokenOutDecimals: 18,
      priceOutAnswer: 300000000000n, // $3000
      priceOutDecimals: 8,
      slippageBps: 100, // 1%
    });
    // 3000 / 3000 = 1 WETH -> 1e18. 1% slip -> 0.99 WETH
    assert.equal(minOut, 990000000000000000n);
  });

  it("throws on invalid inputs", () => {
    const base = {
      amountIn: 1000000000000000000n,
      tokenInDecimals: 18,
      priceInAnswer: 300000000000n,
      priceInDecimals: 8,
      tokenOutDecimals: 6,
      priceOutAnswer: 100000000n,
      priceOutDecimals: 8,
      slippageBps: 50,
    };

    assert.throws(
      () => calculateMinAmountOut({ ...base, amountIn: -1n }),
      /amountIn cannot be negative/,
    );
    assert.throws(
      () => calculateMinAmountOut({ ...base, priceInAnswer: 0n }),
      /priceInAnswer must be > 0/,
    );
    assert.throws(
      () => calculateMinAmountOut({ ...base, priceOutAnswer: -100n }),
      /priceOutAnswer must be > 0/,
    );
    assert.throws(
      () => calculateMinAmountOut({ ...base, slippageBps: 10001 }),
      /slippageBps must be/,
    );
    assert.throws(() => calculateMinAmountOut({ ...base, slippageBps: -1 }), /slippageBps must be/);
    assert.throws(
      () => calculateMinAmountOut({ ...base, tokenInDecimals: -1 }),
      /invalid tokenInDecimals/,
    );
  });
});
