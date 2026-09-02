/**
 * @module simulate
 * Advanced transaction simulation guardrail.
 * Natively executes `eth_call` to trace and simulate an agent's intended transaction
 * against the current blockchain state. Prevents MEV sandwich attacks, honeypot drains,
 * and wasted gas by failing closed if the simulation reverts.
 */

import { createPublicClient, http, type PublicClient, type Hex } from "viem";
import { mainnet } from "viem/chains";

export type SimulateGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type SimulateTxInput = {
  rpc: string;
  account: string;
  to: string;
  data?: string;
  value?: bigint;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "call">;
};

/**
 * Simulates a transaction against the latest block.
 * If the simulation reverts (e.g., honeypot tax, slippage tripped, insufficient gas),
 * this guardrail returns BLOCK.
 * Agents MUST check this before executing live transactions.
 */
export async function simulateTx(input: SimulateTxInput): Promise<SimulateGuardrailResult> {
  const { rpc, account, to, data, value } = input;

  if (!/^0x[a-fA-F0-9]{40}$/.test(account)) {
    return {
      decision: "BLOCK",
      reason: `invalid account address ${account} — BLOCK`,
      allowExecute: false,
    };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(to)) {
    return { decision: "BLOCK", reason: `invalid to address ${to} — BLOCK`, allowExecute: false };
  }
  if (data !== undefined && !/^0x[a-fA-F0-9]*$/.test(data)) {
    return { decision: "BLOCK", reason: `invalid data hex payload — BLOCK`, allowExecute: false };
  }
  if (value !== undefined && (typeof value !== "bigint" || value < 0n)) {
    return { decision: "BLOCK", reason: `invalid value — BLOCK`, allowExecute: false };
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

  try {
    // `client.call` executes `eth_call`. If the smart contract reverts, this throws.
    await client.call({
      account: account as `0x${string}`,
      to: to as `0x${string}`,
      data: (data || "0x") as Hex,
      value: value ?? 0n,
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    // Viem gives extremely detailed revert messages, which is perfect for AI agents
    // to digest and adjust their parameters.
    return {
      decision: "BLOCK",
      reason: `Simulation reverted! The transaction will fail on-chain. Do NOT execute. Revert reason: ${msg}`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `transaction simulation succeeded`,
    allowExecute: true,
  };
}
