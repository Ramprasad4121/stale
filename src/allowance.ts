/**
 * @module allowance
 * Strict guardrails against infinite ERC20 approvals.
 * One of the most common ways AI agents (and users) lose funds is by blindly approving
 * MaxUint256 to a vulnerable or malicious router.
 */

export const MAX_UINT256 =
  115792089237316195423570985008687907853269984665640564039457584007913129639935n;

export type AllowanceGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckApprovalInput = {
  token: string;
  spender: string;
  amount: bigint;
};

/**
 * Validates an ERC20 `approve(spender, amount)` intent.
 * Fails closed (BLOCK) if the agent is attempting an infinite approval (MaxUint256)
 * or a dangerously large approval (e.g. > 2^255).
 * Agents should strictly approve EXACT amounts for immediate execution.
 */
export function checkApproval(input: CheckApprovalInput): AllowanceGuardrailResult {
  const { token, spender, amount } = input;

  if (typeof amount !== "bigint") {
    return {
      decision: "BLOCK",
      reason: "invalid amount type — BLOCK (fail closed)",
      allowExecute: false,
    };
  }
  if (amount < 0n) {
    return {
      decision: "BLOCK",
      reason: "negative allowance — BLOCK (fail closed)",
      allowExecute: false,
    };
  }

  if (!/^0x[a-fA-F0-9]{40}$/.test(token)) {
    return {
      decision: "BLOCK",
      reason: `invalid token address ${token} — BLOCK`,
      allowExecute: false,
    };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(spender)) {
    return {
      decision: "BLOCK",
      reason: `invalid spender address ${spender} — BLOCK`,
      allowExecute: false,
    };
  }

  if (amount === MAX_UINT256) {
    return {
      decision: "BLOCK",
      reason: `infinite approval (MaxUint256) to spender ${spender} is strictly forbidden — BLOCK`,
      allowExecute: false,
    };
  }

  // Block suspiciously large allowances (> 2^255)
  const DANGEROUSLY_LARGE =
    57896044618658097711785492504343953926634992332820282019728792003956564819968n; // 2^255
  if (amount > DANGEROUSLY_LARGE) {
    return {
      decision: "BLOCK",
      reason: `dangerously large approval to spender ${spender} — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `approval to ${spender} is a safe exact amount`,
    allowExecute: true,
  };
}

import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

const allowanceAbi = parseAbi([
  "function allowance(address owner, address spender) view returns (uint256)",
] as const);

export type CheckAllowanceInput = {
  rpc: string;
  token: string;
  owner: string;
  spender: string;
  requiredAmount: bigint;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "readContract">;
};

/**
 * Validates that an agent has granted sufficient ERC20 allowance to a spender
 * *before* attempting a transaction that requires it.
 * Prevents transaction reverts and wasted gas.
 */
export async function checkAllowance(
  input: CheckAllowanceInput,
): Promise<AllowanceGuardrailResult> {
  const { rpc, token, owner, spender, requiredAmount } = input;

  if (typeof requiredAmount !== "bigint" || requiredAmount < 0n) {
    return {
      decision: "BLOCK",
      reason: "invalid requiredAmount — BLOCK (fail closed)",
      allowExecute: false,
    };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(token)) {
    return {
      decision: "BLOCK",
      reason: `invalid token address ${token} — BLOCK`,
      allowExecute: false,
    };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(owner)) {
    return {
      decision: "BLOCK",
      reason: `invalid owner address ${owner} — BLOCK`,
      allowExecute: false,
    };
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(spender)) {
    return {
      decision: "BLOCK",
      reason: `invalid spender address ${spender} — BLOCK`,
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

  let currentAllowance: bigint;
  try {
    currentAllowance = await client.readContract({
      address: token as `0x${string}`,
      abi: allowanceAbi,
      functionName: "allowance",
      args: [owner as `0x${string}`, spender as `0x${string}`],
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      decision: "BLOCK",
      reason: `failed to read allowance — BLOCK (fail closed): ${msg}`,
      allowExecute: false,
    };
  }

  if (currentAllowance < requiredAmount) {
    return {
      decision: "BLOCK",
      reason: `insufficient allowance: owner ${owner} has only approved ${currentAllowance.toString()} < required ${requiredAmount.toString()} for spender ${spender} — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `allowance strictly meets required amount`,
    allowExecute: true,
  };
}
