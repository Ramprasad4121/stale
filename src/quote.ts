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

// Fail closed: bad answer, decimals, or amount throw. Caller (checkPrice) converts to BLOCK.
export function quoteFromFeed({ answer, decimals, amountEth }: QuoteInput): QuoteResult {
  if (typeof answer !== "bigint") {
    throw new Error("answer must be bigint");
  }
  if (answer <= 0n) {
    throw new Error(`invalid answer ${answer.toString()}`);
  }

  let dec: number;
  if (typeof decimals === "bigint") dec = Number(decimals);
  else if (typeof decimals === "number") dec = decimals;
  else throw new Error("decimals must be number or bigint");

  if (!Number.isInteger(dec) || dec < 0 || dec > 36) {
    throw new Error(`invalid decimals ${String(decimals)}`);
  }

  if (amountEth !== undefined && amountEth !== null) {
    if (typeof amountEth !== "number" || !Number.isFinite(amountEth) || amountEth < 0) {
      throw new Error(`invalid amountEth ${String(amountEth)}`);
    }
  }

  const priceUsdStr = formatUnits(answer, dec);
  const priceUsd = Number(priceUsdStr);
  if (!Number.isFinite(priceUsd) || priceUsd < 0) {
    throw new Error(`invalid priceUsd ${priceUsdStr}`);
  }

  const quoteUsd = amountEth !== undefined && amountEth !== null ? amountEth * priceUsd : null;
  if (quoteUsd !== null && (!Number.isFinite(quoteUsd) || quoteUsd < 0)) {
    throw new Error(`invalid quoteUsd ${String(quoteUsd)}`);
  }

  return { priceUsd, quoteUsd };
}
