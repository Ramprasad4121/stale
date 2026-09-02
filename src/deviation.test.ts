import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkPriceDeviation } from "./deviation.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const FEED_A = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"; // ETH/USD
const FEED_B = "0xCfE54B5cD566aB89272946F602D76Ea879CAb4a8"; // another ETH/USD

function mockDeviationClient(
  ansA: bigint,
  decA: number,
  ansB: bigint,
  decB: number,
  error?: Error,
): Pick<PublicClient, "readContract"> {
  let callIndex = 0;
  return {
    readContract: (async ({ functionName }: any) => {
      if (error) throw error;
      callIndex++;
      if (functionName === "latestRoundData") {
        const ans = callIndex <= 2 ? ansA : ansB;
        return [1n, ans, 0n, BigInt(Math.floor(Date.now() / 1000)), 1n];
      }
      if (functionName === "decimals") {
        return callIndex <= 2 ? decA : decB;
      }
    }) as unknown as PublicClient["readContract"],
  };
}

describe("checkPriceDeviation", () => {
  it("ALLOWs when feeds agree within threshold", async () => {
    // Both report ~2500 USD (8 decimals), deviation < 1%
    const res = await checkPriceDeviation({
      rpc: RPC,
      feedA: FEED_A,
      feedB: FEED_B,
      maxDeviationPercent: 2,
      __client: mockDeviationClient(250000000000n, 8, 250500000000n, 8),
    });
    assert.equal(res.decision, "ALLOW");
    assert.ok(res.deviationPercent !== undefined && res.deviationPercent < 2);
  });

  it("BLOCKs when feeds deviate beyond threshold", async () => {
    // A=2500, B=2700 → ~7.7% deviation
    const res = await checkPriceDeviation({
      rpc: RPC,
      feedA: FEED_A,
      feedB: FEED_B,
      maxDeviationPercent: 2,
      __client: mockDeviationClient(250000000000n, 8, 270000000000n, 8),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /ORACLE DEVIATION DANGER/);
  });

  it("BLOCKs on non-positive price", async () => {
    const res = await checkPriceDeviation({
      rpc: RPC,
      feedA: FEED_A,
      feedB: FEED_B,
      maxDeviationPercent: 2,
      __client: mockDeviationClient(0n, 8, 250000000000n, 8),
    });
    assert.equal(res.decision, "BLOCK");
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkPriceDeviation({
      rpc: RPC,
      feedA: FEED_A,
      feedB: FEED_B,
      maxDeviationPercent: 2,
      __client: mockDeviationClient(0n, 8, 0n, 8, new Error("rpc down")),
    });
    assert.equal(res.decision, "BLOCK");
  });

  it("BLOCKs on invalid input", async () => {
    assert.equal(
      (
        await checkPriceDeviation({
          rpc: RPC,
          feedA: "0x123",
          feedB: FEED_B,
          maxDeviationPercent: 2,
        })
      ).decision,
      "BLOCK",
    );
    assert.equal(
      (
        await checkPriceDeviation({
          rpc: RPC,
          feedA: FEED_A,
          feedB: "0x123",
          maxDeviationPercent: 2,
        })
      ).decision,
      "BLOCK",
    );
    assert.equal(
      (
        await checkPriceDeviation({
          rpc: RPC,
          feedA: FEED_A,
          feedB: FEED_B,
          maxDeviationPercent: -1,
        })
      ).decision,
      "BLOCK",
    );
    assert.equal(
      (await checkPriceDeviation({ rpc: "", feedA: FEED_A, feedB: FEED_B, maxDeviationPercent: 2 }))
        .decision,
      "BLOCK",
    );
  });
});
