/**
 * @module sequencer
 * Provides L2 sequencer liveness checks using Chainlink Data Feeds.
 */

import type { PublicClient } from "viem";
import { SEQUENCER_FEEDS } from "./feeds.js";

/**
 * Validates the L2 Sequencer uptime using the official Chainlink Uptime Feed.
 *
 * If a network's sequencer goes down, Chainlink oracles on that network may stop updating.
 * To prevent consuming stale data, we must proactively check the sequencer status.
 *
 * @param chainId - The chain ID of the network being checked (e.g., 42161 for Arbitrum).
 * @param client - A viem PublicClient to read the sequencer contract.
 * @param feedAbi - The ABI for the Chainlink Data Feed contract.
 * @param now - The current time in seconds (for grace period calculation).
 * @returns An error message string if the sequencer is down or in grace period, otherwise null.
 */
export async function checkSequencer(
  chainId: number,
  client: Pick<PublicClient, "readContract">,
  feedAbi: readonly unknown[],
  now: number,
): Promise<string | null> {
  const sequencerFeed = SEQUENCER_FEEDS[chainId];
  if (!sequencerFeed) {
    return null; // Not an L2, or no sequencer feed configured
  }

  try {
    const seqRoundData = (await client.readContract({
      address: sequencerFeed as `0x${string}`,
      abi: feedAbi,
      functionName: "latestRoundData",
    })) as readonly [bigint, bigint, bigint, bigint, bigint];

    const seqAnswer = seqRoundData[1];
    const seqStartedAt = seqRoundData[2];

    if (seqAnswer === 1n) {
      return `L2 Sequencer is DOWN on chain ${chainId} — BLOCK`;
    }

    const GRACE_PERIOD = 3600; // Chainlink recommended 1 hour grace period
    if (seqAnswer === 0n) {
      const timeSinceUp = now - Number(seqStartedAt);
      if (timeSinceUp < GRACE_PERIOD) {
        return `L2 Sequencer is in grace period (${timeSinceUp}s < ${GRACE_PERIOD}s) on chain ${chainId} — BLOCK`;
      }
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return `failed to read sequencer feed on chain ${chainId} — BLOCK (fail closed): ${msg}`;
  }

  return null;
}
