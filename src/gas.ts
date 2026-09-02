/**
 * @module gas
 * Network gas spike circuit breakers.
 * Protects AI agents from executing during extreme network congestion,
 * preventing them from burning disproportionate amounts of their treasury on transaction fees.
 */

import { createPublicClient, http, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

export type GasGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckGasPriceInput = {
  rpc: string;
  /** The absolute maximum gas price (in Gwei) the agent is allowed to pay */
  maxGasPriceGwei: number | bigint;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "getGasPrice">;
};

/**
 * Checks the current network gas price.
 * Fails closed (BLOCK) if the RPC fails or if the current network gas price
 * exceeds the agent's safe `maxGasPriceGwei` threshold.
 */
export async function checkGasPrice(input: CheckGasPriceInput): Promise<GasGuardrailResult> {
  const { rpc, maxGasPriceGwei } = input;

  if (typeof maxGasPriceGwei !== "number" && typeof maxGasPriceGwei !== "bigint") {
    return {
      decision: "BLOCK",
      reason: "invalid maxGasPriceGwei type — BLOCK (fail closed)",
      allowExecute: false,
    };
  }

  const maxGwei = BigInt(maxGasPriceGwei);
  if (maxGwei <= 0n) {
    return {
      decision: "BLOCK",
      reason: "maxGasPriceGwei must be > 0 — BLOCK (fail closed)",
      allowExecute: false,
    };
  }

  if (typeof rpc !== "string" || rpc.trim() === "") {
    return { decision: "BLOCK", reason: "missing rpc — BLOCK (fail closed)", allowExecute: false };
  }

  const client =
    input.__client ??
    createPublicClient({
      chain: mainnet,
      transport: http(rpc),
    });

  let currentGasPrice: bigint;
  try {
    currentGasPrice = await client.getGasPrice();
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      decision: "BLOCK",
      reason: `failed to fetch network gas price — BLOCK (fail closed): ${msg}`,
      allowExecute: false,
    };
  }

  // Convert network gas price (wei) to gwei for comparison. 1 Gwei = 10^9 wei.
  const currentGwei = currentGasPrice / 1000000000n;

  if (currentGwei > maxGwei) {
    return {
      decision: "BLOCK",
      reason: `network gas price ${currentGwei} gwei exceeds maximum allowed ${maxGwei} gwei — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `network gas price ${currentGwei} gwei is within safe limits`,
    allowExecute: true,
  };
}
