import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkPoolV3, checkPoolV2 } from "./dex.js";

const POOL = "0x88e6a0c2ddd26feeb64f039a2c41296fcb3f5640"; // mainnet usdc/eth
const RPC = "https://ethereum-rpc.publicnode.com";

function mockV3Client(liquidity: bigint | Error): Pick<PublicClient, "readContract"> {
  return {
    readContract: (async ({ functionName }: { functionName: string }) => {
      if (functionName === "liquidity") {
        if (liquidity instanceof Error) throw liquidity;
        return liquidity;
      }
      throw new Error(`unexpected ${functionName}`);
    }) as unknown as PublicClient["readContract"],
  };
}

function mockV2Client(r0: bigint | Error, r1?: bigint): Pick<PublicClient, "readContract"> {
  return {
    readContract: (async ({ functionName }: { functionName: string }) => {
      if (functionName === "getReserves") {
        if (r0 instanceof Error) throw r0;
        return [r0, r1 ?? 0n, 1234567890n] as const;
      }
      throw new Error(`unexpected ${functionName}`);
    }) as unknown as PublicClient["readContract"],
  };
}

describe("checkPoolV3", () => {
  it("ALLOWs when liquidity >= minLiquidity", async () => {
    const res = await checkPoolV3({
      rpc: RPC,
      pool: POOL,
      minLiquidity: 1000n,
      __client: mockV3Client(1500n),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs when liquidity < minLiquidity", async () => {
    const res = await checkPoolV3({
      rpc: RPC,
      pool: POOL,
      minLiquidity: 1000n,
      __client: mockV3Client(500n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /active liquidity 500 < required 1000/);
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkPoolV3({
      rpc: RPC,
      pool: POOL,
      minLiquidity: 1000n,
      __client: mockV3Client(new Error("RPC timeout")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to read liquidity/);
  });

  it("BLOCKs on invalid pool address", async () => {
    const res = await checkPoolV3({
      rpc: RPC,
      pool: "0xinvalid",
      minLiquidity: 1000n,
      __client: mockV3Client(1500n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /invalid pool address/);
  });

  it("BLOCKs on invalid minLiquidity type", async () => {
    const res = await checkPoolV3({
      rpc: RPC,
      pool: POOL,
      minLiquidity: -1n,
      __client: mockV3Client(1500n),
    });
    assert.equal(res.decision, "BLOCK");
  });

  it("BLOCKs on empty rpc", async () => {
    const res = await checkPoolV3({
      rpc: "",
      pool: POOL,
      minLiquidity: 1000n,
      __client: mockV3Client(1500n),
    });
    assert.equal(res.decision, "BLOCK");
  });
});

describe("checkPoolV2", () => {
  it("ALLOWs when reserves >= minReserves", async () => {
    const res = await checkPoolV2({
      rpc: RPC,
      pool: POOL,
      minReserve0: 1000n,
      minReserve1: 2000n,
      __client: mockV2Client(1500n, 2500n),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs when reserve0 < minReserve0", async () => {
    const res = await checkPoolV2({
      rpc: RPC,
      pool: POOL,
      minReserve0: 1000n,
      minReserve1: 2000n,
      __client: mockV2Client(500n, 2500n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /reserve0 500 < required 1000/);
  });

  it("BLOCKs when reserve1 < minReserve1", async () => {
    const res = await checkPoolV2({
      rpc: RPC,
      pool: POOL,
      minReserve0: 1000n,
      minReserve1: 2000n,
      __client: mockV2Client(1500n, 1000n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /reserve1 1000 < required 2000/);
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkPoolV2({
      rpc: RPC,
      pool: POOL,
      minReserve0: 1000n,
      minReserve1: 2000n,
      __client: mockV2Client(new Error("RPC timeout")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to read reserves/);
  });

  it("BLOCKs on invalid input", async () => {
    const r1 = await checkPoolV2({ rpc: RPC, pool: "0x1", minReserve0: 1000n, minReserve1: 2000n });
    assert.equal(r1.decision, "BLOCK");

    // @ts-expect-error
    const r2 = await checkPoolV2({ rpc: RPC, pool: POOL, minReserve0: -1n, minReserve1: 2000n });
    assert.equal(r2.decision, "BLOCK");

    // @ts-expect-error
    const r3 = await checkPoolV2({ rpc: RPC, pool: POOL, minReserve0: 1000n, minReserve1: -1n });
    assert.equal(r3.decision, "BLOCK");

    const r4 = await checkPoolV2({ rpc: "", pool: POOL, minReserve0: 1000n, minReserve1: 2000n });
    assert.equal(r4.decision, "BLOCK");
  });
});
