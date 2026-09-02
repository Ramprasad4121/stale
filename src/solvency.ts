/**
 * @module solvency
 * Pre-execution balance and solvency guardrails.
 * Protects AI agents from spamming failing transactions by ensuring they have
 * strict solvency (enough native or ERC20 balance) before attempting to execute.
 */

import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

const erc20Abi = parseAbi(["function balanceOf(address account) view returns (uint256)"] as const);

export type SolvencyGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckBalanceInput = {
  rpc: string;
  agent: string;
  /** The ERC20 token address. If omitted, checks native ETH/gas token balance. */
  token?: string;
  requiredAmount: bigint;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "getBalance" | "readContract">;
};

/**
 * Validates that an agent has strictly enough balance (Native ETH or ERC20)
 * to fulfill a trade intent.
 * Fails closed (BLOCK) on RPC errors or insolvency.
 */
export async function checkBalance(input: CheckBalanceInput): Promise<SolvencyGuardrailResult> {
  const { rpc, agent, token, requiredAmount } = input;

  if (typeof requiredAmount !== "bigint" || requiredAmount < 0n) {
    return {
      decision: "BLOCK",
      reason: "invalid requiredAmount — BLOCK (fail closed)",
      allowExecute: false,
    };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(agent)) {
    return {
      decision: "BLOCK",
      reason: `invalid agent address ${agent} — BLOCK`,
      allowExecute: false,
    };
  }
  if (token !== undefined && !/^0x[a-fA-F0-9]{40}$/.test(token)) {
    return {
      decision: "BLOCK",
      reason: `invalid token address ${token} — BLOCK`,
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

  let balance: bigint;
  try {
    if (token) {
      balance = await client.readContract({
        address: token as `0x${string}`,
        abi: erc20Abi,
        functionName: "balanceOf",
        args: [agent as `0x${string}`],
      });
    } else {
      balance = await client.getBalance({
        address: agent as `0x${string}`,
      });
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    const asset = token ? `token ${token}` : "native balance";
    return {
      decision: "BLOCK",
      reason: `failed to fetch ${asset} for agent ${agent} — BLOCK (fail closed): ${msg}`,
      allowExecute: false,
    };
  }

  if (balance < requiredAmount) {
    const asset = token ? token : "native ETH";
    return {
      decision: "BLOCK",
      reason: `insolvent: agent ${agent} has ${balance.toString()} < required ${requiredAmount.toString()} of ${asset} — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `agent strictly solvent`,
    allowExecute: true,
  };
}
