import { cre, getNetwork, type Runtime, type CronPayload } from "@chainlink/cre-sdk";
import { z } from "zod";
import type { Address } from "viem";
import { PriceFeedAggregator } from "../../contracts/evm/ts/generated/PriceFeedAggregator.js";
import { isStale } from "../../lib/isStale.js";
import { quoteFromFeed } from "../../lib/quote.js";

// ---------- Config — copied keys from read-data-feeds-ts template, single feed for stale ----------

export const configSchema = z.object({
  schedule: z.string(),
  chainName: z.string(),
  feed: z.string(),
  maxAgeSeconds: z.number().int().min(0),
  amountEth: z.number().min(0).optional().nullable(),
});

type Config = z.infer<typeof configSchema>;

// ---------- Helpers ----------

function getEvmClient(chainName: string) {
  const net = getNetwork({
    chainFamily: "evm",
    chainSelectorName: chainName,
    isTestnet: false,
  });
  if (!net) throw new Error(`Network not found for chain name: ${chainName}`);
  return new cre.capabilities.EVMClient(net.chainSelector.selector);
}

const safeJsonStringify = (obj: unknown) =>
  JSON.stringify(obj, (_, v) => (typeof v === "bigint" ? v.toString() : v), 2);

// ---------- Core ----------

function buildBlockResult(
  feed: string,
  maxAgeSeconds: number,
  now: number,
  reason: string,
  partial?: { answer?: string; updatedAt?: string; ageSeconds?: number | null; priceUsd?: number | null; quoteUsd?: number | null; amountEth?: number | null },
) {
  return {
    decision: "BLOCK" as const,
    reason,
    feed,
    answer: partial?.answer ?? "0",
    priceUsd: partial?.priceUsd ?? null,
    amountEth: partial?.amountEth ?? null,
    quoteUsd: partial?.quoteUsd ?? null,
    updatedAt: partial?.updatedAt ?? "0",
    ageSeconds: partial?.ageSeconds ?? null,
    maxAgeSeconds,
    now,
    allowExecute: false,
    execute: { action: "none", note: "dry-run. no tx. no Agents call." },
  };
}

// ---------- Handler — single cron trigger, as template ----------

export function onCron(runtime: Runtime<Config>, _payload: CronPayload): string {
  const cfg = runtime.config;
  const feed = cfg.feed as Address;
  const maxAgeSeconds = cfg.maxAgeSeconds;
  const amountEth = cfg.amountEth ?? null;
  const now = Math.floor(Date.now() / 1000);

  // Fail closed on bad config
  if (!/^0x[a-fA-F0-9]{40}$/.test(feed)) {
    return safeJsonStringify(buildBlockResult(feed, maxAgeSeconds, now, `invalid feed ${feed} — BLOCK`));
  }

  const evmClient = getEvmClient(cfg.chainName);
  const aggregator = new PriceFeedAggregator(evmClient, feed);

  let roundId: bigint;
  let answer: bigint;
  let updatedAt: bigint;
  let answeredInRound: bigint;
  let decimals: number;
  try {
    decimals = aggregator.decimals(runtime);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return safeJsonStringify(buildBlockResult(feed, maxAgeSeconds, now, `failed to read decimals() — BLOCK: ${msg}`.slice(0, 200)));
  }

  try {
    const data = aggregator.latestRoundData(runtime);
    // latestRoundData returns [roundId, answer, startedAt, updatedAt, answeredInRound]
    roundId = data[0];
    answer = data[1];
    updatedAt = data[3];
    answeredInRound = data[4];
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return safeJsonStringify(buildBlockResult(feed, maxAgeSeconds, now, `failed to read latestRoundData — BLOCK: ${msg}`.slice(0, 200)));
  }

  if (updatedAt === 0n) {
    return safeJsonStringify(
      buildBlockResult(feed, maxAgeSeconds, now, `updatedAt is 0 (no data) — BLOCK`, {
        answer: answer.toString(),
        updatedAt: updatedAt.toString(),
        ageSeconds: null,
      }),
    );
  }
  if (answer <= 0n) {
    return safeJsonStringify(
      buildBlockResult(feed, maxAgeSeconds, now, `answer is ${answer.toString()} (invalid price) — BLOCK`, {
        answer: answer.toString(),
        updatedAt: updatedAt.toString(),
        ageSeconds: null,
      }),
    );
  }
  if (answeredInRound < roundId) {
    return safeJsonStringify(
      buildBlockResult(feed, maxAgeSeconds, now, `incomplete round: answeredInRound ${answeredInRound.toString()} < roundId ${roundId.toString()} (unanswered round) — BLOCK`, {
        answer: answer.toString(),
        updatedAt: updatedAt.toString(),
        ageSeconds: null,
      }),
    );
  }

  const stale = isStale({ updatedAt, nowSeconds: now, maxAgeSeconds });

  let priceUsd: number | null = null;
  let quoteUsd: number | null = null;
  try {
    const q = quoteFromFeed({ answer, decimals, amountEth });
    priceUsd = q.priceUsd;
    quoteUsd = q.quoteUsd;
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return safeJsonStringify({
      decision: "BLOCK",
      reason: `quote failed: ${msg} — BLOCK`.slice(0, 200),
      feed,
      answer: answer.toString(),
      priceUsd: null,
      amountEth,
      quoteUsd: null,
      updatedAt: updatedAt.toString(),
      ageSeconds: stale.ageSeconds,
      maxAgeSeconds,
      now,
      allowExecute: false,
      execute: { action: "none", note: "dry-run. no tx. no Agents call." },
    });
  }

  const allowExecute = stale.decision === "ALLOW";
  const result = {
    decision: stale.decision,
    reason: stale.reason.slice(0, 200),
    feed,
    answer: answer.toString(),
    priceUsd,
    amountEth,
    quoteUsd,
    updatedAt: updatedAt.toString(),
    ageSeconds: stale.ageSeconds,
    maxAgeSeconds,
    now,
    allowExecute,
    execute: allowExecute
      ? { action: "none", note: "v3 dry-run. no tx. no Agents call." }
      : { action: "none", note: "skipped — BLOCK" },
  };

  // No EVM write. No wallet. Dry-run only.
  runtime.log(`stale | ${result.decision} | age=${result.ageSeconds}s maxAge=${maxAgeSeconds}s priceUsd=${priceUsd} quoteUsd=${quoteUsd}`);
  return safeJsonStringify(result);
}

// ---------- Init — one trigger only (cron), as template ----------

export function initWorkflow(config: Config) {
  const cron = new cre.capabilities.CronCapability();
  return [cre.handler(cron.trigger({ schedule: config.schedule }), onCron)];
}
