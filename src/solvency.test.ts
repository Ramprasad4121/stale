import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkBalance } from "./solvency.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const AGENT = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"; // vitalik.eth
const TOKEN = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC

function mockSolvencyClient(
  bal: bigint | Error,
): Pick<PublicClient, "getBalance" | "readContract"> {
  return {
    getBalance: async () => {
      if (bal instanceof Error) throw bal;
      return bal;
    },
    readContract: (async ({ functionName }: { functionName: string }) => {
      if (functionName === "balanceOf") {
        if (bal instanceof Error) throw bal;
        return bal;
      }
      throw new Error("unexpected");
    }) as unknown as PublicClient["readContract"],
  };
}

describe("checkBalance", () => {
  it("ALLOWs when native balance >= required", async () => {
    const res = await checkBalance({
      rpc: RPC,
      agent: AGENT,
      requiredAmount: 100n,
      __client: mockSolvencyClient(150n),
    });
    assert.equal(res.decision, "ALLOW");
  });

  it("BLOCKs when native balance < required", async () => {
    const res = await checkBalance({
      rpc: RPC,
      agent: AGENT,
      requiredAmount: 100n,
      __client: mockSolvencyClient(50n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /insolvent/);
  });

  it("ALLOWs when token balance >= required", async () => {
    const res = await checkBalance({
      rpc: RPC,
      agent: AGENT,
      token: TOKEN,
      requiredAmount: 100n,
      __client: mockSolvencyClient(150n),
    });
    assert.equal(res.decision, "ALLOW");
  });

  it("BLOCKs when token balance < required", async () => {
    const res = await checkBalance({
      rpc: RPC,
      agent: AGENT,
      token: TOKEN,
      requiredAmount: 100n,
      __client: mockSolvencyClient(50n),
    });
    assert.equal(res.decision, "BLOCK");
  });

  it("BLOCKs on RPC error", async () => {
    const res = await checkBalance({
      rpc: RPC,
      agent: AGENT,
      requiredAmount: 100n,
      __client: mockSolvencyClient(new Error("rpc error")),
    });
    assert.equal(res.decision, "BLOCK");
  });

  it("BLOCKs on invalid inputs", async () => {
    assert.equal(
      (await checkBalance({ rpc: RPC, agent: "0x123", requiredAmount: 100n })).decision,
      "BLOCK",
    );
    assert.equal(
      (await checkBalance({ rpc: RPC, agent: AGENT, token: "0x123", requiredAmount: 100n }))
        .decision,
      "BLOCK",
    );
    // @ts-expect-error
    assert.equal(
      (await checkBalance({ rpc: RPC, agent: AGENT, requiredAmount: -1n })).decision,
      "BLOCK",
    );
    assert.equal(
      (await checkBalance({ rpc: "", agent: AGENT, requiredAmount: 100n })).decision,
      "BLOCK",
    );
  });
});
