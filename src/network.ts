/**
 * @module network
 * Network & RPC State Guardrails.
 * Protects agents from acting on out-of-sync RPC nodes, cross-chain misconfigurations,
 * and local state desyncs (nonce mismatches).
 */

import { createPublicClient, http, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

export type NetworkGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckRpcSyncInput = {
  rpc: string;
  /** The maximum acceptable age of the latest block in seconds (e.g. 60) */
  maxBlockAgeSeconds: number;
  /** @internal inject mock client */
  __client?: Pick<PublicClient, "getBlock">;
};

export type CheckChainIdInput = {
  rpc: string;
  expectedChainId: number;
  /** @internal inject mock client */
  __client?: Pick<PublicClient, "getChainId">;
};

export type CheckNonceInput = {
  rpc: string;
  agent: string;
  /** The exact nonce the agent expects to use. Blocks if the network nonce is higher (already used). */
  expectedNonce: number;
  /** @internal inject mock client */
  __client?: Pick<PublicClient, "getTransactionCount">;
};

/**
 * Validates that the RPC node is actively syncing and not stalled.
 * Fails closed (BLOCK) if the latest block timestamp is older than `maxBlockAgeSeconds`.
 * This prevents agents from acting on horribly outdated state from a stalled RPC.
 */
export async function checkRpcSync(input: CheckRpcSyncInput): Promise<NetworkGuardrailResult> {
  const { rpc, maxBlockAgeSeconds } = input;

  if (typeof maxBlockAgeSeconds !== "number" || maxBlockAgeSeconds <= 0) {
    return { decision: "BLOCK", reason: "invalid maxBlockAgeSeconds — BLOCK", allowExecute: false };
  }
  if (typeof rpc !== "string" || rpc.trim() === "") {
    return { decision: "BLOCK", reason: "missing rpc — BLOCK", allowExecute: false };
  }

  const client = input.__client ?? createPublicClient({ chain: mainnet, transport: http(rpc) });

  let block;
  try {
    block = await client.getBlock({ blockTag: "latest" });
  } catch (err) {
    return {
      decision: "BLOCK",
      reason: `failed to fetch latest block — BLOCK: ${err}`,
      allowExecute: false,
    };
  }

  const nowSeconds = Math.floor(Date.now() / 1000);
  const blockTime = Number(block.timestamp);
  const age = nowSeconds - blockTime;

  if (age > maxBlockAgeSeconds) {
    return {
      decision: "BLOCK",
      reason: `RPC STALL DANGER: latest block is ${age} seconds old (max ${maxBlockAgeSeconds}). The RPC is out of sync. — BLOCK`,
      allowExecute: false,
    };
  }

  return { decision: "ALLOW", reason: `RPC is synced (block age ${age}s)`, allowExecute: true };
}

/**
 * Validates that the RPC node connects to the expected blockchain network.
 * Prevents catastrophic cross-chain replay attacks and misconfigurations.
 */
export async function checkChainId(input: CheckChainIdInput): Promise<NetworkGuardrailResult> {
  const { rpc, expectedChainId } = input;

  if (typeof expectedChainId !== "number" || expectedChainId <= 0) {
    return { decision: "BLOCK", reason: "invalid expectedChainId — BLOCK", allowExecute: false };
  }

  const client = input.__client ?? createPublicClient({ chain: mainnet, transport: http(rpc) });

  let currentChainId;
  try {
    currentChainId = await client.getChainId();
  } catch (err) {
    return {
      decision: "BLOCK",
      reason: `failed to fetch chain id — BLOCK: ${err}`,
      allowExecute: false,
    };
  }

  if (currentChainId !== expectedChainId) {
    return {
      decision: "BLOCK",
      reason: `CHAIN MISMATCH DANGER: expected chain ${expectedChainId}, but RPC is on chain ${currentChainId}. — BLOCK`,
      allowExecute: false,
    };
  }

  return { decision: "ALLOW", reason: `RPC chain ID matches expected`, allowExecute: true };
}

/**
 * Validates the agent's on-chain nonce to prevent double-spending or acting out-of-sync.
 * Fails closed (BLOCK) if the network nonce is strictly greater than the agent's expected nonce.
 */
export async function checkNonce(input: CheckNonceInput): Promise<NetworkGuardrailResult> {
  const { rpc, agent, expectedNonce } = input;

  if (!/^0x[a-fA-F0-9]{40}$/.test(agent)) {
    return {
      decision: "BLOCK",
      reason: `invalid agent address ${agent} — BLOCK`,
      allowExecute: false,
    };
  }
  if (typeof expectedNonce !== "number" || expectedNonce < 0) {
    return { decision: "BLOCK", reason: "invalid expectedNonce — BLOCK", allowExecute: false };
  }

  const client = input.__client ?? createPublicClient({ chain: mainnet, transport: http(rpc) });

  let networkNonce;
  try {
    networkNonce = await client.getTransactionCount({ address: agent as `0x${string}` });
  } catch (err) {
    return {
      decision: "BLOCK",
      reason: `failed to fetch nonce — BLOCK: ${err}`,
      allowExecute: false,
    };
  }

  if (networkNonce > expectedNonce) {
    return {
      decision: "BLOCK",
      reason: `STATE DESYNC: network nonce (${networkNonce}) is higher than expected (${expectedNonce}). Previous transactions have confirmed. — BLOCK`,
      allowExecute: false,
    };
  }

  return { decision: "ALLOW", reason: `nonce is in sync`, allowExecute: true };
}
