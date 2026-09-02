/**
 * @module deviation
 * Multi-Oracle Price Deviation Guard.
 *
 * Compares two independent Chainlink price feeds for the same asset pair.
 * If the prices deviate beyond a configurable threshold, the oracle data is
 * likely manipulated or one feed is stale — BLOCK.
 *
 * This is the gold-standard defense against oracle manipulation attacks
 * (e.g. flash-loan-driven price spikes on a single oracle).
 */

import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

const feedAbi = parseAbi([
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
  "function decimals() view returns (uint8)",
] as const);

export type DeviationGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
  /** Deviation percentage between the two feeds */
  deviationPercent?: number;
};

export type CheckPriceDeviationInput = {
  rpc: string;
  /** First Chainlink feed address */
  feedA: string;
  /** Second Chainlink feed address (independent source for same pair) */
  feedB: string;
  /** Maximum acceptable deviation between feeds as a percentage (e.g. 2 = 2%) */
  maxDeviationPercent: number;
  /** @internal inject mock client */
  __client?: Pick<PublicClient, "readContract">;
};

/**
 * Queries two independent Chainlink feeds and compares their prices.
 * Fails closed (BLOCK) if the deviation exceeds the threshold or on any error.
 */
export async function checkPriceDeviation(
  input: CheckPriceDeviationInput,
): Promise<DeviationGuardrailResult> {
  const { rpc, feedA, feedB, maxDeviationPercent } = input;

  if (!/^0x[a-fA-F0-9]{40}$/.test(feedA)) {
    return { decision: "BLOCK", reason: `invalid feedA address — BLOCK`, allowExecute: false };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(feedB)) {
    return { decision: "BLOCK", reason: `invalid feedB address — BLOCK`, allowExecute: false };
  }
  if (typeof maxDeviationPercent !== "number" || maxDeviationPercent <= 0) {
    return {
      decision: "BLOCK",
      reason: "invalid maxDeviationPercent — BLOCK",
      allowExecute: false,
    };
  }
  if (typeof rpc !== "string" || rpc.trim() === "") {
    return { decision: "BLOCK", reason: "missing rpc — BLOCK", allowExecute: false };
  }

  const client = input.__client ?? createPublicClient({ chain: mainnet, transport: http(rpc) });

  try {
    const [roundA, decA, roundB, decB] = await Promise.all([
      client.readContract({
        address: feedA as `0x${string}`,
        abi: feedAbi,
        functionName: "latestRoundData",
      }),
      client.readContract({
        address: feedA as `0x${string}`,
        abi: feedAbi,
        functionName: "decimals",
      }),
      client.readContract({
        address: feedB as `0x${string}`,
        abi: feedAbi,
        functionName: "latestRoundData",
      }),
      client.readContract({
        address: feedB as `0x${string}`,
        abi: feedAbi,
        functionName: "decimals",
      }),
    ]);

    const answerA = roundA[1];
    const answerB = roundB[1];
    const decimalsA = decA;
    const decimalsB = decB;

    if (answerA <= 0n || answerB <= 0n) {
      return {
        decision: "BLOCK",
        reason: `one or both feeds returned non-positive price — BLOCK`,
        allowExecute: false,
      };
    }

    // Normalize both prices to 18 decimals for comparison
    const normalizedA = answerA * 10n ** BigInt(18 - decimalsA);
    const normalizedB = answerB * 10n ** BigInt(18 - decimalsB);

    // Calculate deviation: |A - B| / avg(A, B) * 100
    const diff = normalizedA > normalizedB ? normalizedA - normalizedB : normalizedB - normalizedA;
    const avg = (normalizedA + normalizedB) / 2n;
    // Multiply by 10000 first for precision, then divide
    const deviationBps = Number((diff * 10000n) / avg);
    const deviationPercent = deviationBps / 100;

    if (deviationPercent > maxDeviationPercent) {
      return {
        decision: "BLOCK",
        reason: `ORACLE DEVIATION DANGER: feeds deviate by ${deviationPercent.toFixed(2)}% (max ${maxDeviationPercent}%). Possible oracle manipulation. — BLOCK`,
        allowExecute: false,
        deviationPercent,
      };
    }

    return {
      decision: "ALLOW",
      reason: `feeds agree within ${deviationPercent.toFixed(2)}% deviation`,
      allowExecute: true,
      deviationPercent,
    };
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      decision: "BLOCK",
      reason: `failed to query feeds — BLOCK (fail closed): ${msg}`,
      allowExecute: false,
    };
  }
}
