/**
 * @module mev
 * Strict MEV-Protection Guardrail.
 * Ensures AI agents do not broadcast trades to the public mempool where they
 * will be ruthlessly sandwiched and front-run by MEV searchers.
 */

export type MevGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckMevRpcInput = {
  rpc: string;
};

/**
 * A curated list of known, trusted MEV-protected RPC endpoints.
 * These RPCs route transactions directly to block builders, bypassing the public mempool.
 */
export const MEV_PROTECTED_RPCS = [
  "https://rpc.flashbots.net",
  "https://rpc.mevblocker.io",
  "https://eth.rpc.blxrbdn.com",
  "https://rpc.beaverbuild.org",
  "https://rpc.titanbuilder.xyz",
  "https://api.edennetwork.io/v1/rpc",
];

/**
 * Validates that the agent is using a private, MEV-protected RPC endpoint.
 * Fails closed (BLOCK) if the RPC is public (e.g., Alchemy, Infura) or unknown.
 */
export function checkMevRpc(input: CheckMevRpcInput): MevGuardrailResult {
  const { rpc } = input;

  if (typeof rpc !== "string" || rpc.trim() === "") {
    return { decision: "BLOCK", reason: "missing rpc — BLOCK", allowExecute: false };
  }

  try {
    const url = new URL(rpc);
    const isProtected = MEV_PROTECTED_RPCS.some((protectedRpc) => {
      const pUrl = new URL(protectedRpc);
      return url.hostname === pUrl.hostname;
    });

    if (!isProtected) {
      return {
        decision: "BLOCK",
        reason: `PUBLIC MEMPOOL DANGER: RPC ${url.hostname} is not a recognized MEV-protected endpoint. Transactions will be sandwiched. — BLOCK`,
        allowExecute: false,
      };
    }

    return {
      decision: "ALLOW",
      reason: `RPC ${url.hostname} is MEV-protected`,
      allowExecute: true,
    };
  } catch (err) {
    return {
      decision: "BLOCK",
      reason: `invalid rpc url format — BLOCK`,
      allowExecute: false,
    };
  }
}
