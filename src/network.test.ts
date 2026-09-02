import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkRpcSync, checkChainId, checkNonce } from "./network.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const AGENT = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";

describe("network guardrails", () => {
  describe("checkRpcSync", () => {
    const mockSyncClient = (timestamp: bigint | Error) =>
      ({
        getBlock: async () => {
          if (timestamp instanceof Error) throw timestamp;
          return { timestamp };
        },
      }) as unknown as Pick<PublicClient, "getBlock">;

    it("ALLOWs if block is fresh", async () => {
      const now = BigInt(Math.floor(Date.now() / 1000));
      const res = await checkRpcSync({
        rpc: RPC,
        maxBlockAgeSeconds: 60,
        __client: mockSyncClient(now - 10n),
      });
      assert.equal(res.decision, "ALLOW");
    });

    it("BLOCKs if block is stalled", async () => {
      const now = BigInt(Math.floor(Date.now() / 1000));
      const res = await checkRpcSync({
        rpc: RPC,
        maxBlockAgeSeconds: 60,
        __client: mockSyncClient(now - 100n),
      });
      assert.equal(res.decision, "BLOCK");
      assert.match(res.reason, /RPC STALL DANGER/);
    });

    it("BLOCKs on invalid input or RPC error", async () => {
      assert.equal(
        (await checkRpcSync({ rpc: RPC, maxBlockAgeSeconds: 0, __client: mockSyncClient(10n) }))
          .decision,
        "BLOCK",
      );
      assert.equal(
        (await checkRpcSync({ rpc: "", maxBlockAgeSeconds: 60, __client: mockSyncClient(10n) }))
          .decision,
        "BLOCK",
      );
      assert.equal(
        (
          await checkRpcSync({
            rpc: RPC,
            maxBlockAgeSeconds: 60,
            __client: mockSyncClient(new Error("rpc error")),
          })
        ).decision,
        "BLOCK",
      );
    });
  });

  describe("checkChainId", () => {
    const mockChainClient = (chainId: number | Error) =>
      ({
        getChainId: async () => {
          if (chainId instanceof Error) throw chainId;
          return chainId;
        },
      }) as Pick<PublicClient, "getChainId">;

    it("ALLOWs if chain matches", async () => {
      const res = await checkChainId({
        rpc: RPC,
        expectedChainId: 1,
        __client: mockChainClient(1),
      });
      assert.equal(res.decision, "ALLOW");
    });

    it("BLOCKs if chain mismatch", async () => {
      const res = await checkChainId({
        rpc: RPC,
        expectedChainId: 1,
        __client: mockChainClient(42161),
      });
      assert.equal(res.decision, "BLOCK");
      assert.match(res.reason, /CHAIN MISMATCH DANGER/);
    });

    it("BLOCKs on invalid input or RPC error", async () => {
      assert.equal(
        (await checkChainId({ rpc: RPC, expectedChainId: -1, __client: mockChainClient(1) }))
          .decision,
        "BLOCK",
      );
      assert.equal(
        (
          await checkChainId({
            rpc: RPC,
            expectedChainId: 1,
            __client: mockChainClient(new Error("rpc error")),
          })
        ).decision,
        "BLOCK",
      );
    });
  });

  describe("checkNonce", () => {
    const mockNonceClient = (nonce: number | Error) =>
      ({
        getTransactionCount: async () => {
          if (nonce instanceof Error) throw nonce;
          return nonce;
        },
      }) as Pick<PublicClient, "getTransactionCount">;

    it("ALLOWs if nonce is synced", async () => {
      const res = await checkNonce({
        rpc: RPC,
        agent: AGENT,
        expectedNonce: 5,
        __client: mockNonceClient(5),
      });
      assert.equal(res.decision, "ALLOW");

      const res2 = await checkNonce({
        rpc: RPC,
        agent: AGENT,
        expectedNonce: 5,
        __client: mockNonceClient(4),
      });
      assert.equal(res2.decision, "ALLOW"); // network is slightly behind or agent expects to broadcast next, fine
    });

    it("BLOCKs if network nonce is higher (state desync)", async () => {
      const res = await checkNonce({
        rpc: RPC,
        agent: AGENT,
        expectedNonce: 5,
        __client: mockNonceClient(6),
      });
      assert.equal(res.decision, "BLOCK");
      assert.match(res.reason, /STATE DESYNC/);
    });

    it("BLOCKs on invalid input or RPC error", async () => {
      assert.equal(
        (await checkNonce({ rpc: RPC, agent: "0x123", expectedNonce: 5 })).decision,
        "BLOCK",
      );
      assert.equal(
        (await checkNonce({ rpc: RPC, agent: AGENT, expectedNonce: -1 })).decision,
        "BLOCK",
      );
      assert.equal(
        (
          await checkNonce({
            rpc: RPC,
            agent: AGENT,
            expectedNonce: 5,
            __client: mockNonceClient(new Error("rpc err")),
          })
        ).decision,
        "BLOCK",
      );
    });
  });
});
