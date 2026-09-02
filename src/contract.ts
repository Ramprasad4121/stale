/**
 * @module contract
 * EOA Phishing Guardrail.
 * Prevents AI agents from interacting with or approving tokens to Externally Owned Accounts (EOAs),
 * which is a common phishing vector (e.g. an agent is tricked into thinking a scammer's wallet is a DEX Router).
 */

import { createPublicClient, http, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

export type ContractGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckIsContractInput = {
  rpc: string;
  address: string;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "getBytecode">;
};

/**
 * Validates that a target address is a deployed smart contract, not an EOA.
 * Fails closed (BLOCK) if the address has no bytecode or if the RPC fails.
 */
export async function checkIsContract(
  input: CheckIsContractInput,
): Promise<ContractGuardrailResult> {
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

  let bytecode;
  try {
    bytecode = await client.getBytecode({
      address: address as `0x${string}`,
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return {
      decision: "BLOCK",
      reason: `failed to fetch bytecode for ${address} — BLOCK (fail closed): ${msg}`,
      allowExecute: false,
    };
  }

  if (!bytecode || bytecode === "0x") {
    return {
      decision: "BLOCK",
      reason: `PHISHING DANGER: address ${address} is an EOA (Externally Owned Account) with no bytecode. Do not approve or route funds to EOAs. — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `address ${address} is a deployed smart contract`,
    allowExecute: true,
  };
}
