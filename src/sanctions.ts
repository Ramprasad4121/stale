/**
 * @module sanctions
 * OFAC Sanctions Guardrail utilizing Chainlink's Sanctions Oracle.
 * Ensures agents remain compliant and do not interact with flagged/sanctioned entities.
 */

import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

// Mainnet Chainlink OFAC Sanctions Oracle
export const SANCTIONS_ORACLE = "0x40C57923924B5c5c5455c48D93317139ADDaC8fb";

const sanctionsAbi = parseAbi(["function isSanctioned(address addr) view returns (bool)"] as const);

export type SanctionsGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckSanctionsInput = {
  rpc: string;
  address: string;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "readContract">;
};

/**
 * Validates whether a target address (or the agent itself) is on the OFAC Sanctions list
 * using the Chainlink Sanctions Oracle.
 * Fails closed (BLOCK) if the address is sanctioned or if the RPC fails.
 */
export async function checkSanctioned(
  input: CheckSanctionsInput,
): Promise<SanctionsGuardrailResult> {
  const { rpc, address } = input;

  if (!/^0x[a-fA-F0-9]{40}$/.test(address)) {
    return { decision: "BLOCK", reason: `invalid address ${address} — BLOCK`, allowExecute: false };
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

  let isSanctioned = false;
  try {
    isSanctioned = await client.readContract({
      address: SANCTIONS_ORACLE,
      abi: sanctionsAbi,
      functionName: "isSanctioned",
      args: [address as `0x${string}`],
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      decision: "BLOCK",
      reason: `failed to query sanctions oracle for ${address} — BLOCK (fail closed): ${msg}`,
      allowExecute: false,
    };
  }

  if (isSanctioned) {
    return {
      decision: "BLOCK",
      reason: `COMPLIANCE VIOLATION: address ${address} is heavily sanctioned — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `address is compliant`,
    allowExecute: true,
  };
}
