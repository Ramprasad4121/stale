// keep in sync with src/quote.ts
import { formatUnits } from "viem";

export type QuoteInput = {
  answer: bigint;
  decimals: number | bigint;
  amountEth?: number | null;
};

export type QuoteResult = {
  priceUsd: number;
  quoteUsd: number | null;
};

/**
 * Price math from official Data Feed `answer` + `decimals`.
 * Fail closed: bad answer/decimals/amount throw — caller converts to BLOCK.
 * Uses viem `formatUnits` per https://viem.sh/docs/utilities/formatUnits
 */
export function quoteFromFeed({ answer, decimals, amountEth }: QuoteInput): QuoteResult {
  if (typeof answer !== "bigint") {
    throw new Error("answer must be bigint");
  }
  if (answer <= 0n) {
    throw new Error(`invalid answer ${answer.toString()}`);
  }

  const dec = typeof decimals === "bigint" ? Number(decimals) : decimals;
  if (typeof dec !== "number" || !Number.isInteger(dec) || dec < 0 || dec > 36) {
    throw new Error(`invalid decimals ${String(decimals)}`);
  }

  if (amountEth !== undefined && amountEth !== null) {
    if (typeof amountEth !== "number" || !Number.isFinite(amountEth) || amountEth < 0) {
      throw new Error(`invalid amountEth ${String(amountEth)}`);
    }
  }

  // viem formatUnits: string value scaled by decimals, e.g. 245377000000n + 8 → "2453.77"
  const priceUsd = Number(formatUnits(answer, dec));
  if (!Number.isFinite(priceUsd) || priceUsd < 0) {
    throw new Error(`invalid priceUsd ${String(priceUsd)}`);
  }

  const quoteUsd = amountEth != null ? amountEth * priceUsd : null;
  if (quoteUsd !== null && (!Number.isFinite(quoteUsd) || quoteUsd < 0)) {
    throw new Error(`invalid quoteUsd ${String(quoteUsd)}`);
  }

  return { priceUsd, quoteUsd };
}
