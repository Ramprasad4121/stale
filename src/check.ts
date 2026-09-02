import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";
import { isStale } from "./isStale.js";
import { quoteFromFeed } from "./quote.js";
import { lookupFeed, SEQUENCER_FEEDS } from "./feeds.js";

/**
 * Official Data Feed ABI — only `decimals` and `latestRoundData` per
 * https://docs.chain.link/data-feeds/api-reference
 */
const feedAbi = parseAbi([
  "function decimals() view returns (uint8)",
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
] as const);

export type CheckPriceInput = {
  rpc: string;
  feed: string;
  maxAgeSeconds: number;
  amountEth?: number | null;
  nowSeconds?: number;
  /** @internal inject mock client for tests (no live RPC) — viem-compatible */
  __client?: Pick<PublicClient, "readContract"> & { getChainId?: PublicClient["getChainId"] };
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

/**
 * Full check: viem `latestRoundData` + `decimals` → `isStale` → `quoteFromFeed`.
 * Fail closed on any RPC/read/zero/negative/invalid. No wallet, no write.
 * Verified against viem `createPublicClient` + `readContract` docs.
 */
export async function checkPrice(input: CheckPriceInput): Promise<CheckPriceResult> {
  const { rpc, feed, maxAgeSeconds, amountEth } = input;
  const now = input.nowSeconds ?? Math.floor(Date.now() / 1000);

  if (amountEth !== undefined && amountEth !== null) {
    if (typeof amountEth !== "number" || !Number.isFinite(amountEth) || amountEth < 0) {
      return blockResult(input, `invalid amountEth ${String(amountEth)} — BLOCK (fail closed)`);
    }
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(feed)) {
    return blockResult(input, `invalid feed ${feed} — BLOCK`);
  }

  const feedEntry = lookupFeed(feed);
  if (!feedEntry) {
    return blockResult(input, `unknown/unsupported feed ${feed} — BLOCK`);
  }
  if (!Number.isFinite(maxAgeSeconds) || !Number.isInteger(maxAgeSeconds) || maxAgeSeconds < 0) {
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

  // Chain binding: registry entry's chainId must match RPC chainId
  // Skip getChainId when __client is injected and mock does not implement it (existing tests)
  const maybeGetChainId = (client as unknown as { getChainId?: () => Promise<number> }).getChainId;
  if (typeof maybeGetChainId === "function") {
    try {
      const chainId = await maybeGetChainId.call(client);
      if (chainId !== feedEntry.chainId) {
        return blockResult(
          input,
          `chainId mismatch: feed ${feed} is chainId ${feedEntry.chainId} but rpc returned chainId ${String(chainId)} — BLOCK`,
        );
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return blockResult(input, `failed to get chainId — BLOCK (fail closed): ${msg}`);
    }
  }

  // Sequencer Liveness Check (Fail closed)
  const sequencerFeed = SEQUENCER_FEEDS[feedEntry.chainId];
  if (sequencerFeed) {
    try {
      const seqRoundData = await client.readContract({
        address: sequencerFeed as `0x${string}`,
        abi: feedAbi,
        functionName: "latestRoundData",
      });

      const seqAnswer = seqRoundData[1];
      const seqStartedAt = seqRoundData[2];

      if (seqAnswer === 1n) {
        return blockResult(input, `L2 Sequencer is DOWN on chain ${feedEntry.chainId} — BLOCK`);
      }

      const GRACE_PERIOD = 3600;
      if (seqAnswer === 0n) {
        const timeSinceUp = now - Number(seqStartedAt);
        if (timeSinceUp < GRACE_PERIOD) {
          return blockResult(
            input,
            `L2 Sequencer is in grace period (${timeSinceUp}s < ${GRACE_PERIOD}s) on chain ${feedEntry.chainId} — BLOCK`,
          );
        }
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return blockResult(
        input,
        `failed to read sequencer feed on chain ${feedEntry.chainId} — BLOCK (fail closed): ${msg}`,
      );
    }
  }

  // Parallel reads — viem batches `eth_call` at end of tick (public client docs)
  let roundId: bigint;
  let answer: bigint;
  let updatedAt: bigint;
  let answeredInRound: bigint;
  let decimals: number;

  try {
    const [roundData, dec] = await Promise.all([
      client.readContract({
        address: feed as `0x${string}`,
        abi: feedAbi,
        functionName: "latestRoundData",
      }) as Promise<readonly [bigint, bigint, bigint, bigint, bigint]>,
      client.readContract({
        address: feed as `0x${string}`,
        abi: feedAbi,
        functionName: "decimals",
      }) as Promise<number | bigint>,
    ]);

    const data = roundData;
    roundId = data[0];
    answer = data[1];
    updatedAt = data[3];
    answeredInRound = data[4];

    const d = dec;
    decimals = typeof d === "bigint" ? Number(d) : d;
    if (!Number.isInteger(decimals) || decimals < 0 || decimals > 36) {
      throw new Error(`invalid decimals ${String(d)}`);
    }

    if (updatedAt === 0n) {
      return blockResult(input, `updatedAt is 0 (no data) from ${feed} — BLOCK`, {
        answer: answer.toString(),
        updatedAt: updatedAt.toString(),
        ageSeconds: null,
      });
    }
    if (answer <= 0n) {
      return blockResult(
        input,
        `answer is ${answer.toString()} (invalid price) from ${feed} — BLOCK`,
        {
          answer: answer.toString(),
          updatedAt: updatedAt.toString(),
          ageSeconds: null,
        },
      );
    }
    if (answeredInRound < roundId) {
      return blockResult(
        input,
        `incomplete round: answeredInRound ${answeredInRound.toString()} < roundId ${roundId.toString()} (unanswered round) from ${feed} — BLOCK`,
        {
          answer: answer.toString(),
          updatedAt: updatedAt.toString(),
          ageSeconds: null,
        },
      );
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    // Distinguish which call failed by message; keep generic fail-closed
    if (msg.includes("decimals")) {
      return blockResult(
        input,
        `failed to read decimals() from ${feed} — BLOCK (fail closed): ${msg}`,
      );
    }
    return blockResult(
      input,
      `failed to read latestRoundData from ${feed} — BLOCK (fail closed): ${msg}`,
    );
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
