/**
 * @module dex
 * Provides on-chain DEX pool liquidity guardrails to prevent AI agents from
 * executing trades in illiquid honeypot pools and suffering massive slippage.
 */

import { createPublicClient, http, parseAbi, type PublicClient } from "viem";
import { mainnet } from "viem/chains";

const v3Abi = parseAbi(["function liquidity() view returns (uint128)"] as const);

const v2Abi = parseAbi([
  "function getReserves() view returns (uint112 _reserve0, uint112 _reserve1, uint32 _blockTimestampLast)",
] as const);

export type DexGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  pool: string;
  allowExecute: boolean;
};

function blockDex(pool: string, reason: string): DexGuardrailResult {
  return {
    decision: "BLOCK",
    reason,
    pool,
    allowExecute: false,
  };
}

function allowDex(pool: string, reason: string): DexGuardrailResult {
  return {
    decision: "ALLOW",
    reason,
    pool,
    allowExecute: true,
  };
}

export type CheckPoolV3Input = {
  rpc: string;
  pool: string;
  minLiquidity: bigint;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "readContract">;
};

/**
 * Checks a Uniswap V3 (or clone) pool to ensure its active in-range liquidity
 * meets a minimum safe threshold before an agent executes a swap.
 */
export async function checkPoolV3(input: CheckPoolV3Input): Promise<DexGuardrailResult> {
  const { rpc, pool, minLiquidity } = input;

  if (typeof minLiquidity !== "bigint" || minLiquidity < 0n) {
    return blockDex(pool, `invalid minLiquidity ${String(minLiquidity)} — BLOCK (fail closed)`);
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(pool)) {
    return blockDex(pool, `invalid pool address ${pool} — BLOCK`);
  }
  if (typeof rpc !== "string" || rpc.trim() === "") {
    return blockDex(pool, "missing rpc — BLOCK (fail closed)");
  }

  const client =
    input.__client ??
    createPublicClient({
      chain: mainnet, // chain doesn't matter for raw RPC calls without chain assertions
      transport: http(rpc),
    });

  let activeLiquidity: bigint;
  try {
    activeLiquidity = await client.readContract({
      address: pool as `0x${string}`,
      abi: v3Abi,
      functionName: "liquidity",
    });
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return blockDex(
      pool,
      `failed to read liquidity from V3 pool ${pool} — BLOCK (fail closed): ${msg}`,
    );
  }

  if (activeLiquidity < minLiquidity) {
    return blockDex(
      pool,
      `active liquidity ${activeLiquidity.toString()} < required ${minLiquidity.toString()} on V3 pool ${pool} — BLOCK`,
    );
  }

  return allowDex(
    pool,
    `V3 pool ${pool} liquidity ${activeLiquidity.toString()} meets minimum requirements`,
  );
}

export type CheckPoolV2Input = {
  rpc: string;
  pool: string;
  minReserve0: bigint;
  minReserve1: bigint;
  /** @internal inject mock client for tests */
  __client?: Pick<PublicClient, "readContract">;
};

/**
 * Checks a Uniswap V2 (or clone) pool to ensure both token reserves
 * meet a minimum safe threshold before an agent executes a swap.
 */
export async function checkPoolV2(input: CheckPoolV2Input): Promise<DexGuardrailResult> {
  const { rpc, pool, minReserve0, minReserve1 } = input;

  if (typeof minReserve0 !== "bigint" || minReserve0 < 0n) {
    return blockDex(pool, `invalid minReserve0 ${String(minReserve0)} — BLOCK (fail closed)`);
  }
  if (typeof minReserve1 !== "bigint" || minReserve1 < 0n) {
    return blockDex(pool, `invalid minReserve1 ${String(minReserve1)} — BLOCK (fail closed)`);
  }
  if (!/^0x[a-fA-F0-9]{40}$/.test(pool)) {
    return blockDex(pool, `invalid pool address ${pool} — BLOCK`);
  }
  if (typeof rpc !== "string" || rpc.trim() === "") {
    return blockDex(pool, "missing rpc — BLOCK (fail closed)");
  }

  const client =
    input.__client ??
    createPublicClient({
      chain: mainnet,
      transport: http(rpc),
    });

  let reserve0: bigint;
  let reserve1: bigint;
  try {
    const reserves = await client.readContract({
      address: pool as `0x${string}`,
      abi: v2Abi,
      functionName: "getReserves",
    });
    reserve0 = reserves[0];
    reserve1 = reserves[1];
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return blockDex(
      pool,
      `failed to read reserves from V2 pool ${pool} — BLOCK (fail closed): ${msg}`,
    );
  }

  if (reserve0 < minReserve0) {
    return blockDex(
      pool,
      `reserve0 ${reserve0.toString()} < required ${minReserve0.toString()} on V2 pool ${pool} — BLOCK`,
    );
  }

  if (reserve1 < minReserve1) {
    return blockDex(
      pool,
      `reserve1 ${reserve1.toString()} < required ${minReserve1.toString()} on V2 pool ${pool} — BLOCK`,
    );
  }

  return allowDex(pool, `V2 pool ${pool} reserves meet minimum requirements`);
}
