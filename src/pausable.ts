/**
 * @module pausable
 * Protocol Paused State checking.
 * Protects agents from attempting to interact with paused protocols (e.g. USDC, Aave).
 */

import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

const pausableAbi = parseAbi(["function paused() view returns (bool)"] as const);

export type PausableGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckPausedInput = {
  rpc: string;
  contract: string;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "readContract">;
};

/**
 * Checks if a contract implements the standard `paused() view returns (bool)` pattern,
 * and if so, blocks execution if the contract is paused.
 * If the contract does not implement `paused()`, it safely defaults to ALLOW.
 */
export async function checkPaused(input: CheckPausedInput): Promise<PausableGuardrailResult> {
  const { rpc, contract } = input;

  if (!/^0x[a-fA-F0-9]{40}$/.test(contract)) {
    return {
      decision: "BLOCK",
      reason: `invalid contract address ${contract} — BLOCK`,
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

  let isPaused = false;
  try {
    isPaused = await client.readContract({
      address: contract as `0x${string}`,
      abi: pausableAbi,
      functionName: "paused",
    });
  } catch (err) {
    // If the call reverts or method is not found, the contract likely isn't Pausable.
    // In this specific guardrail, reverting defaults to ALLOW because not all contracts have paused().
    return {
      decision: "ALLOW",
      reason: `contract ${contract} does not implement paused() or RPC failed — safely ALLOW`,
      allowExecute: true,
    };
  }

  if (isPaused) {
    return {
      decision: "BLOCK",
      reason: `contract ${contract} is currently PAUSED — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `contract ${contract} is active (not paused)`,
    allowExecute: true,
  };
}
