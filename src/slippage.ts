/**
 * @module slippage
 * Provides exact bigint math to calculate safe `minAmountOut` bounds for DEX trades
 * using Chainlink Price Feeds. This acts as an absolute guardrail against MEV sandwich attacks.
 */

export type CalculateMinAmountOutInput = {
  amountIn: bigint;
  tokenInDecimals: number;
  priceInAnswer: bigint;
  priceInDecimals: number;

  tokenOutDecimals: number;
  priceOutAnswer: bigint;
  priceOutDecimals: number;

  /** Slippage tolerance in basis points. E.g. 50 = 0.5% */
  slippageBps: number;
};

/**
 * Calculates a safe minAmountOut for a token swap using Chainlink Price Feeds.
 * Uses 100% bigint math to prevent floating point precision loss.
 *
 * Fails closed (throws) if inputs are invalid or negative.
 */
export function calculateMinAmountOut(input: CalculateMinAmountOutInput): bigint {
  const {
    amountIn,
    tokenInDecimals,
    priceInAnswer,
    priceInDecimals,
    tokenOutDecimals,
    priceOutAnswer,
    priceOutDecimals,
    slippageBps,
  } = input;

  if (amountIn < 0n) throw new Error("amountIn cannot be negative");
  if (priceInAnswer <= 0n) throw new Error("priceInAnswer must be > 0");
  if (priceOutAnswer <= 0n) throw new Error("priceOutAnswer must be > 0");

  if (!Number.isInteger(tokenInDecimals) || tokenInDecimals < 0)
    throw new Error("invalid tokenInDecimals");
  if (!Number.isInteger(priceInDecimals) || priceInDecimals < 0)
    throw new Error("invalid priceInDecimals");
  if (!Number.isInteger(tokenOutDecimals) || tokenOutDecimals < 0)
    throw new Error("invalid tokenOutDecimals");
  if (!Number.isInteger(priceOutDecimals) || priceOutDecimals < 0)
    throw new Error("invalid priceOutDecimals");

  if (!Number.isInteger(slippageBps) || slippageBps < 0 || slippageBps > 10000) {
    throw new Error("slippageBps must be an integer between 0 and 10000");
  }

  // Formula:
  // value of amountIn (base units scaled by 10^(tokenInDec + priceInDec)) = amountIn * priceInAnswer
  // value of 1 tokenOut (base units scaled by 10^(tokenOutDec + priceOutDec)) = priceOutAnswer
  //
  // rawAmountOut = (amountIn * priceInAnswer * 10^(tokenOutDecimals + priceOutDecimals)) /
  //                (priceOutAnswer * 10^(tokenInDecimals + priceInDecimals))

  const numerator = amountIn * priceInAnswer * 10n ** BigInt(tokenOutDecimals + priceOutDecimals);
  const denominator = priceOutAnswer * 10n ** BigInt(tokenInDecimals + priceInDecimals);

  const rawAmountOut = numerator / denominator;

  const minAmountOut = (rawAmountOut * BigInt(10000 - slippageBps)) / 10000n;

  return minAmountOut;
}
