import { createPublicClient, http, parseAbi } from "viem";
import { mainnet } from "viem/chains";
import { isStale } from "./isStale.js";
import { quoteFromFeed } from "./quote.js";

const feedAbi = parseAbi([
  "function decimals() view returns (uint8)",
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
]);

export type CheckPriceInput = {
  rpc: string;
  feed: string;
  maxAgeSeconds: number;
  amountEth?: number | null;
  nowSeconds?: number;
  // internal: inject mock client for tests (no live RPC)
  __client?: { readContract: (args: any) => Promise<any> };
};

export type CheckPriceResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  feed: string;
  answer: string;
  priceUsd: number | null;
  amountEth: number | null;
  quoteUsd: number | null;
  updatedAt: string;
  ageSeconds: number | null;
  maxAgeSeconds: number;
  now: number;
  allowExecute: boolean;
};

function blockResult(
  input: CheckPriceInput,
  reason: string,
  partial?: Partial<CheckPriceResult>,
): CheckPriceResult {
  const now = input.nowSeconds ?? Math.floor(Date.now() / 1000);
  return {
    decision: "BLOCK",
    reason,
    feed: input.feed,
    answer: partial?.answer ?? "0",
    priceUsd: partial?.priceUsd ?? null,
    amountEth: input.amountEth ?? null,
    quoteUsd: partial?.quoteUsd ?? null,
    updatedAt: partial?.updatedAt ?? "0",
    ageSeconds: partial?.ageSeconds ?? null,
    maxAgeSeconds: input.maxAgeSeconds,
    now,
    allowExecute: false,
  };
}

export async function checkPrice(input: CheckPriceInput): Promise<CheckPriceResult> {
  const { rpc, feed, maxAgeSeconds, amountEth } = input;
  const now = input.nowSeconds ?? Math.floor(Date.now() / 1000);

  // Fail closed on bad amount (also checked in CLI, but keep here for library use)
  if (amountEth !== undefined && amountEth !== null) {
    if (typeof amountEth !== "number" || !Number.isFinite(amountEth) || amountEth < 0) {
      return blockResult(input, `invalid amountEth ${String(amountEth)} — BLOCK (fail closed)`);
    }
  }

  if (!/^0x[a-fA-F0-9]{40}$/.test(feed)) {
    return blockResult(input, `invalid feed ${feed} — BLOCK`);
  }
  if (!Number.isFinite(maxAgeSeconds) || maxAgeSeconds < 0 || !Number.isInteger(maxAgeSeconds)) {
    return blockResult(input, `invalid maxAgeSeconds ${String(maxAgeSeconds)} — BLOCK`);
  }
  if (typeof rpc !== "string" || rpc.trim() === "") {
    return blockResult(input, "missing rpc — BLOCK (fail closed)");
  }

  const client =
    input.__client ??
    createPublicClient({
      chain: mainnet,
      transport: http(rpc),
    });

  let answer: bigint;
  let updatedAt: bigint;
  try {
    const data = (await client.readContract({
      address: feed as `0x${string}`,
      abi: feedAbi,
      functionName: "latestRoundData",
    })) as readonly [bigint, bigint, bigint, bigint, bigint];
    answer = data[1];
    updatedAt = data[3];
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return blockResult(input, `failed to read latestRoundData from ${feed} — BLOCK (fail closed): ${msg}`);
  }

  if (updatedAt === 0n) {
    return blockResult(input, `updatedAt is 0 (no data) from ${feed} — BLOCK`, {
      answer: answer.toString(),
      updatedAt: updatedAt.toString(),
      ageSeconds: null,
    });
  }
  if (answer <= 0n) {
    return blockResult(input, `answer is ${answer.toString()} (invalid price) from ${feed} — BLOCK`, {
      answer: answer.toString(),
      updatedAt: updatedAt.toString(),
      ageSeconds: null,
    });
  }

  let decimals: number;
  try {
    const d = (await client.readContract({
      address: feed as `0x${string}`,
      abi: feedAbi,
      functionName: "decimals",
    })) as number | bigint;
    decimals = typeof d === "bigint" ? Number(d) : d;
    if (!Number.isInteger(decimals) || decimals < 0 || decimals > 36) {
      throw new Error(`invalid decimals ${String(d)}`);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return blockResult(input, `failed to read decimals() from ${feed} — BLOCK (fail closed): ${msg}`, {
      answer: answer.toString(),
      updatedAt: updatedAt.toString(),
      ageSeconds: null,
    });
  }

  const stale = isStale({ updatedAt, nowSeconds: now, maxAgeSeconds });

  let priceUsd: number | null = null;
  let quoteUsd: number | null = null;
  try {
    const q = quoteFromFeed({ answer, decimals, amountEth: amountEth ?? null });
    priceUsd = q.priceUsd;
    quoteUsd = q.quoteUsd;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      decision: "BLOCK",
      reason: `quote failed: ${msg} — BLOCK`,
      feed,
      answer: answer.toString(),
      priceUsd: null,
      amountEth: amountEth ?? null,
      quoteUsd: null,
      updatedAt: updatedAt.toString(),
      ageSeconds: stale.ageSeconds,
      maxAgeSeconds,
      now,
      allowExecute: false,
    };
  }

  const allowExecute = stale.decision === "ALLOW";
  return {
    decision: stale.decision,
    reason: stale.reason,
    feed,
    answer: answer.toString(),
    priceUsd,
    amountEth: amountEth ?? null,
    quoteUsd,
    updatedAt: updatedAt.toString(),
    ageSeconds: stale.ageSeconds,
    maxAgeSeconds,
    now,
    allowExecute,
  };
}
