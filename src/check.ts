import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";
import { isStale } from "./isStale.js";
import { quoteFromFeed } from "./quote.js";
import { lookupFeed } from "./feeds.js";
import { SEQUENCER_FEEDS } from "./feeds.js";
import { checkSequencer } from "./sequencer.js";

/**
 * Official Data Feed ABI — only `decimals` and `latestRoundData` per
 * https://docs.chain.link/data-feeds/api-reference
 */
const feedAbi = parseAbi([
  "function decimals() view returns (uint8)",
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
] as const);

export type CheckPricesInput = {
  rpc: string;
  feeds: Array<{
    feed: string;
    maxAgeSeconds: number;
    amountEth?: number | null;
  }>;
  nowSeconds?: number;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "readContract" | "multicall"> & {
    getChainId?: PublicClient["getChainId"];
  };
};

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

/**
 * Check multiple price feeds in a single RPC multicall.
 * This is heavily optimized for high-frequency agents and rebalancing operations.
 * Fetches latestRoundData, decimals, and sequencer uptime simultaneously.
 */
export async function checkPrices(input: CheckPricesInput): Promise<CheckPriceResult[]> {
  const { rpc, feeds } = input;
  const now = input.nowSeconds ?? Math.floor(Date.now() / 1000);

  // Return empty immediately if no feeds
  if (feeds.length === 0) return [];

  const client =
    input.__client ??
    createPublicClient({
      chain: mainnet,
      transport: http(rpc),
    });

  // 1. Initial validations and registry lookups
  type PreparedFeed = {
    originalIndex: number;
    feed: string;
    maxAgeSeconds: number;
    amountEth: number | null;
    entry: ReturnType<typeof lookupFeed>;
    blockedResult?: CheckPriceResult;
  };

  const prepared: PreparedFeed[] = feeds.map((f, i) => {
    const amountEth = f.amountEth ?? null;
    const baseBlock = {
      rpc,
      feed: f.feed,
      maxAgeSeconds: f.maxAgeSeconds,
      amountEth,
      nowSeconds: now,
      __client: input.__client,
    };

    if (amountEth !== null) {
      if (typeof amountEth !== "number" || !Number.isFinite(amountEth) || amountEth < 0) {
        return {
          originalIndex: i,
          feed: f.feed,
          maxAgeSeconds: f.maxAgeSeconds,
          amountEth,
          entry: null,
          blockedResult: blockResult(
            baseBlock,
            `invalid amountEth ${String(amountEth)} — BLOCK (fail closed)`,
          ),
        };
      }
    }
    if (!/^0x[a-fA-F0-9]{40}$/.test(f.feed)) {
      return {
        originalIndex: i,
        feed: f.feed,
        maxAgeSeconds: f.maxAgeSeconds,
        amountEth,
        entry: null,
        blockedResult: blockResult(baseBlock, `invalid feed ${f.feed} — BLOCK`),
      };
    }
    const entry = lookupFeed(f.feed);
    if (!entry) {
      return {
        originalIndex: i,
        feed: f.feed,
        maxAgeSeconds: f.maxAgeSeconds,
        amountEth,
        entry: null,
        blockedResult: blockResult(baseBlock, `unknown/unsupported feed ${f.feed} — BLOCK`),
      };
    }
    if (
      !Number.isFinite(f.maxAgeSeconds) ||
      !Number.isInteger(f.maxAgeSeconds) ||
      f.maxAgeSeconds < 0
    ) {
      return {
        originalIndex: i,
        feed: f.feed,
        maxAgeSeconds: f.maxAgeSeconds,
        amountEth,
        entry,
        blockedResult: blockResult(
          baseBlock,
          `invalid maxAgeSeconds ${String(f.maxAgeSeconds)} — BLOCK`,
        ),
      };
    }
    if (typeof rpc !== "string" || rpc.trim() === "") {
      return {
        originalIndex: i,
        feed: f.feed,
        maxAgeSeconds: f.maxAgeSeconds,
        amountEth,
        entry,
        blockedResult: blockResult(baseBlock, "missing rpc — BLOCK (fail closed)"),
      };
    }
    return { originalIndex: i, feed: f.feed, maxAgeSeconds: f.maxAgeSeconds, amountEth, entry };
  });

  // Verify all valid feeds belong to the same chain to ensure the RPC matches
  const validFeeds = prepared.filter((p) => !p.blockedResult && p.entry);
  if (validFeeds.length > 0) {
    const targetChainId = validFeeds[0].entry!.chainId;
    for (const p of validFeeds) {
      if (p.entry!.chainId !== targetChainId) {
        p.blockedResult = blockResult(
          {
            rpc,
            feed: p.feed,
            maxAgeSeconds: p.maxAgeSeconds,
            amountEth: p.amountEth,
            nowSeconds: now,
            __client: input.__client,
          },
          `chain mismatch in batch: expected ${targetChainId}, got ${p.entry!.chainId} — BLOCK`,
        );
      }
    }

    // Check RPC chainId
    const maybeGetChainId = (client as unknown as { getChainId?: () => Promise<number> })
      .getChainId;
    if (typeof maybeGetChainId === "function") {
      try {
        const rpcChainId = await maybeGetChainId.call(client);
        for (const p of validFeeds) {
          if (!p.blockedResult && p.entry!.chainId !== rpcChainId) {
            p.blockedResult = blockResult(
              {
                rpc,
                feed: p.feed,
                maxAgeSeconds: p.maxAgeSeconds,
                amountEth: p.amountEth,
                nowSeconds: now,
                __client: input.__client,
              },
              `chainId mismatch: feed ${p.feed} is chainId ${p.entry!.chainId} but rpc returned chainId ${String(rpcChainId)} — BLOCK`,
            );
          }
        }
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        for (const p of validFeeds) {
          if (!p.blockedResult)
            p.blockedResult = blockResult(
              {
                rpc,
                feed: p.feed,
                maxAgeSeconds: p.maxAgeSeconds,
                amountEth: p.amountEth,
                nowSeconds: now,
                __client: input.__client,
              },
              `failed to get chainId — BLOCK (fail closed): ${msg}`,
            );
        }
      }
    }
  }

  const finalResults: CheckPriceResult[] = new Array(feeds.length);
  const stillValid = prepared.filter((p) => !p.blockedResult);

  // If no feeds are still valid, return the blocked ones
  if (stillValid.length === 0) {
    for (const p of prepared) {
      finalResults[p.originalIndex] = p.blockedResult!;
    }
    return finalResults;
  }

  // 2. Prepare Multicall
  const targetChainId = stillValid[0].entry!.chainId;
  // We'll optionally add sequencer call at the end of the contracts array if needed
  const hasSequencer = SEQUENCER_FEEDS[targetChainId] !== undefined;

  const contracts = [];
  for (const p of stillValid) {
    contracts.push({
      address: p.feed as `0x${string}`,
      abi: feedAbi,
      functionName: "latestRoundData",
    });
    contracts.push({ address: p.feed as `0x${string}`, abi: feedAbi, functionName: "decimals" });
  }
  if (hasSequencer) {
    contracts.push({
      address: SEQUENCER_FEEDS[targetChainId] as `0x${string}`,
      abi: feedAbi,
      functionName: "latestRoundData",
    });
  }

  // Execute Multicall
  let multicallResults: any[] = [];
  try {
    multicallResults = await client.multicall({
      contracts,
      allowFailure: true,
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    // Multicall completely failed
    for (const p of stillValid) {
      p.blockedResult = blockResult(
        {
          rpc,
          feed: p.feed,
          maxAgeSeconds: p.maxAgeSeconds,
          amountEth: p.amountEth,
          nowSeconds: now,
          __client: input.__client,
        },
        `multicall failed — BLOCK (fail closed): ${msg}`,
      );
    }
  }

  // Evaluate Sequencer
  let seqErrorMsg: string | null = null;
  if (hasSequencer && multicallResults.length > 0) {
    const seqRes = multicallResults[multicallResults.length - 1];
    if (seqRes.status === "failure") {
      seqErrorMsg = `failed to read sequencer feed on chain ${targetChainId} — BLOCK (fail closed)`;
    } else {
      const seqRoundData = seqRes.result as readonly [bigint, bigint, bigint, bigint, bigint];
      const seqAnswer = seqRoundData[1];
      const seqStartedAt = seqRoundData[2];
      if (seqAnswer === 1n) {
        seqErrorMsg = `L2 Sequencer is DOWN on chain ${targetChainId} — BLOCK`;
      } else if (seqAnswer === 0n) {
        const timeSinceUp = now - Number(seqStartedAt);
        const GRACE_PERIOD = 3600;
        if (timeSinceUp < GRACE_PERIOD) {
          seqErrorMsg = `L2 Sequencer is in grace period (${timeSinceUp}s < ${GRACE_PERIOD}s) on chain ${targetChainId} — BLOCK`;
        }
      }
    }
  }

  // Evaluate individual feeds
  if (multicallResults.length > 0) {
    for (let i = 0; i < stillValid.length; i++) {
      const p = stillValid[i];
      if (p.blockedResult) continue;

      if (seqErrorMsg) {
        p.blockedResult = blockResult(
          {
            rpc,
            feed: p.feed,
            maxAgeSeconds: p.maxAgeSeconds,
            amountEth: p.amountEth,
            nowSeconds: now,
            __client: input.__client,
          },
          seqErrorMsg,
        );
        continue;
      }

      const lrdRes = multicallResults[i * 2];
      const decRes = multicallResults[i * 2 + 1];

      if (lrdRes.status === "failure") {
        p.blockedResult = blockResult(
          {
            rpc,
            feed: p.feed,
            maxAgeSeconds: p.maxAgeSeconds,
            amountEth: p.amountEth,
            nowSeconds: now,
            __client: input.__client,
          },
          `failed to read latestRoundData from ${p.feed} — BLOCK (fail closed)`,
        );
        continue;
      }
      if (decRes.status === "failure") {
        p.blockedResult = blockResult(
          {
            rpc,
            feed: p.feed,
            maxAgeSeconds: p.maxAgeSeconds,
            amountEth: p.amountEth,
            nowSeconds: now,
            __client: input.__client,
          },
          `failed to read decimals() from ${p.feed} — BLOCK (fail closed)`,
        );
        continue;
      }

      const data = lrdRes.result as readonly [bigint, bigint, bigint, bigint, bigint];
      const roundId = data[0];
      const answer = data[1];
      const updatedAt = data[3];
      const answeredInRound = data[4];

      const d = decRes.result;
      const decimals = typeof d === "bigint" ? Number(d) : Number(d);

      const baseBlock = {
        rpc,
        feed: p.feed,
        maxAgeSeconds: p.maxAgeSeconds,
        amountEth: p.amountEth,
        nowSeconds: now,
        __client: input.__client,
      };
      if (!Number.isInteger(decimals) || decimals < 0 || decimals > 36) {
        p.blockedResult = blockResult(
          baseBlock,
          `failed to read decimals() from ${p.feed} — BLOCK (fail closed): invalid decimals ${String(d)}`,
        );
        continue;
      }
      if (updatedAt === 0n) {
        p.blockedResult = blockResult(
          baseBlock,
          `updatedAt is 0 (no data) from ${p.feed} — BLOCK`,
          { answer: answer.toString(), updatedAt: updatedAt.toString(), ageSeconds: null },
        );
        continue;
      }
      if (answer <= 0n) {
        p.blockedResult = blockResult(
          baseBlock,
          `answer is ${answer.toString()} (invalid price) from ${p.feed} — BLOCK`,
          { answer: answer.toString(), updatedAt: updatedAt.toString(), ageSeconds: null },
        );
        continue;
      }
      if (answeredInRound < roundId) {
        p.blockedResult = blockResult(
          baseBlock,
          `incomplete round: answeredInRound ${answeredInRound.toString()} < roundId ${roundId.toString()} (unanswered round) from ${p.feed} — BLOCK`,
          { answer: answer.toString(), updatedAt: updatedAt.toString(), ageSeconds: null },
        );
        continue;
      }

      const stale = isStale({ updatedAt, nowSeconds: now, maxAgeSeconds: p.maxAgeSeconds });

      let priceUsd: number | null = null;
      let quoteUsd: number | null = null;
      try {
        const q = quoteFromFeed({ answer, decimals, amountEth: p.amountEth });
        priceUsd = q.priceUsd;
        quoteUsd = q.quoteUsd;
      } catch (err) {
        const msg = err instanceof Error ? err.message : String(err);
        p.blockedResult = {
          decision: "BLOCK",
          reason: `quote failed: ${msg} — BLOCK`,
          feed: p.feed,
          answer: answer.toString(),
          priceUsd: null,
          amountEth: p.amountEth,
          quoteUsd: null,
          updatedAt: updatedAt.toString(),
          ageSeconds: stale.ageSeconds,
          maxAgeSeconds: p.maxAgeSeconds,
          now,
          allowExecute: false,
        };
        continue;
      }

      p.blockedResult = {
        decision: stale.decision,
        reason: stale.reason,
        feed: p.feed,
        answer: answer.toString(),
        priceUsd,
        amountEth: p.amountEth,
        quoteUsd,
        updatedAt: updatedAt.toString(),
        ageSeconds: stale.ageSeconds,
        maxAgeSeconds: p.maxAgeSeconds,
        now,
        allowExecute: stale.decision === "ALLOW",
      };
    }
  }

  // Populate final array
  for (const p of prepared) {
    finalResults[p.originalIndex] = p.blockedResult!;
  }

  return finalResults;
}

/**
 * Single price check. Convenience wrapper around checkPrices.
 */
export async function checkPrice(input: CheckPriceInput): Promise<CheckPriceResult> {
  const res = await checkPrices({
    rpc: input.rpc,
    feeds: [
      {
        feed: input.feed,
        maxAgeSeconds: input.maxAgeSeconds,
        amountEth: input.amountEth,
      },
    ],
    nowSeconds: input.nowSeconds,
    __client: input.__client as any,
  });
  return res[0];
}
