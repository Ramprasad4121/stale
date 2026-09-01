import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkPrice } from "./check.js";

const FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
const RPC = "https://ethereum-rpc.publicnode.com";
const NOW = 1_724_520_000;

function mockClient(opts: { answer: bigint; updatedAt: bigint; decimals: number | bigint | Error; roundId?: bigint; answeredInRound?: bigint; chainId?: number | Error }): Pick<PublicClient, "readContract"> & { getChainId?: PublicClient["getChainId"] } {
  const client: Pick<PublicClient, "readContract"> & { getChainId?: PublicClient["getChainId"] } = {
    readContract: (async ({ functionName }: { functionName: string }) => {
      if (functionName === "latestRoundData") {
        // (roundId, answer, startedAt, updatedAt, answeredInRound)
        return [opts.roundId ?? 1n, opts.answer, 1n, opts.updatedAt, opts.answeredInRound ?? 1n] as const;
      }
      if (functionName === "decimals") {
        if (opts.decimals instanceof Error) throw opts.decimals;
        return opts.decimals;
      }
      throw new Error(`unexpected ${functionName}`);
    }) as unknown as PublicClient["readContract"],
  };
  if (opts.chainId !== undefined) {
    client.getChainId = (async () => {
      if (opts.chainId instanceof Error) throw opts.chainId;
      return opts.chainId as number;
    }) as unknown as PublicClient["getChainId"];
  }
  return client;
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

  it("incomplete round BLOCK — answeredInRound < roundId with fresh updatedAt", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8, roundId: 2n, answeredInRound: 1n }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.match(r.reason, /answeredInRound.*roundId|incomplete round/);
  });

  it("complete round fresh ALLOW — answeredInRound == roundId", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8, roundId: 5n, answeredInRound: 5n }),
    });
    assert.equal(r.decision, "ALLOW");
    assert.equal(r.allowExecute, true);
    assert.equal(r.ageSeconds, 10);
  });

  it("chainId 1 with default feed fresh ALLOW", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8, chainId: 1 }),
    });
    assert.equal(r.decision, "ALLOW");
    assert.equal(r.allowExecute, true);
    assert.match(r.reason, /fresh/);
  });

  it("chainId mismatch BLOCK — Sepolia rpc with mainnet feed", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8, chainId: 11155111 }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.match(r.reason, /chainId mismatch/);
  });

  it("getChainId throw → BLOCK", async () => {
    const r = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8, chainId: new Error("chainId down") }),
    });
    assert.equal(r.decision, "BLOCK");
    assert.equal(r.allowExecute, false);
    assert.match(r.reason, /failed to get chainId/);
  });

  it("malformed RPC and invalid feed — fail closed", async () => {
    // invalid feed address → BLOCK before RPC
    const rFeed = await checkPrice({
      rpc: RPC,
      feed: "0xbad",
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8 }),
    });
    assert.equal(rFeed.decision, "BLOCK");
    assert.match(rFeed.reason, /invalid feed/);

    // missing rpc → BLOCK
    const rRpc = await checkPrice({
      rpc: "",
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8 }),
    });
    assert.equal(rRpc.decision, "BLOCK");
    assert.match(rRpc.reason, /missing rpc/);

    // malformed readContract throws generic → BLOCK
    const badClient = {
      readContract: (async () => {
        throw new Error("malformed response: unexpected tuple");
      }) as unknown as PublicClient["readContract"],
    };
    const rMal = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: 60,
      nowSeconds: NOW,
      __client: badClient,
    });
    assert.equal(rMal.decision, "BLOCK");
    assert.match(rMal.reason, /failed to read/);

    // invalid maxAge → BLOCK
    const rPolicy = await checkPrice({
      rpc: RPC,
      feed: FEED,
      maxAgeSeconds: NaN,
      nowSeconds: NOW,
      __client: mockClient({ answer: 300000000000n, updatedAt: BigInt(NOW - 10), decimals: 8 }),
    });
    assert.equal(rPolicy.decision, "BLOCK");
    assert.match(rPolicy.reason, /invalid maxAge/);
  });
});
