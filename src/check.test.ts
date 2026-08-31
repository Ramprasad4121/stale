import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { checkPrice } from "./check.js";

const FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
const RPC = "https://ethereum-rpc.publicnode.com";
const NOW = 1_724_520_000;

function mockClient(opts: { answer: bigint; updatedAt: bigint; decimals: number | bigint | Error }) {
  return {
    readContract: async ({ functionName }: { functionName: string }) => {
      if (functionName === "latestRoundData") {
        // (roundId, answer, startedAt, updatedAt, answeredInRound)
        return [1n, opts.answer, 1n, opts.updatedAt, 1n] as const;
      }
      if (functionName === "decimals") {
        if (opts.decimals instanceof Error) throw opts.decimals;
        return opts.decimals;
      }
      throw new Error(`unexpected ${functionName}`);
    },
  };
}

describe("checkPrice (mocked viem, no live RPC)", () => {
  it("fresh ALLOW", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      amountEth: 0.5,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8 }),
    });
    assert.equal(r.decision, "ALLOW");
    assert.equal(r.allowExecute, true);
    assert.equal(r.ageSeconds, 10);
    assert.equal(r.priceUsd, 3000);
    assert.equal(r.quoteUsd, 1500);
    assert.equal(r.answer, "300000000000");
  });

  it("stale BLOCK", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      amountEth: null,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 100), decimals: 8 }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.equal(r.ageSeconds, 100);
  });

  it("updatedAt 0 BLOCK", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: 0n, decimals: 8 }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.match(r.reason, /updatedAt is 0/);
  });

  it("answer <=0 BLOCK", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 0n, updatedAt: BigInt(NOW - 10), decimals: 8 }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.match(r.reason, /answer is 0/);

    const r2 = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: -1n, updatedAt: BigInt(NOW - 10), decimals: 8 }),
    });
    assert.equal(r2.decision, "BLOCK");
  });

  it("decimals throw BLOCK", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: new Error("decimals down") }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.match(r.reason, /decimals/);
  });
});
