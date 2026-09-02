import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkPaused } from "./pausable.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const CONTRACT = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

function mockPausedClient(isPaused: boolean | Error): Pick<PublicClient, "readContract"> {
  return {
    readContract: (async () => {
      if (isPaused instanceof Error) throw isPaused;
      return isPaused;
    }) as unknown as PublicClient["readContract"],
  };
}

describe("checkPaused", () => {
  it("ALLOWs if contract is not paused", async () => {
    const res = await checkPaused({
      rpc: RPC,
      contract: CONTRACT,
      __client: mockPausedClient(false),
    });
    assert.equal(res.decision, "ALLOW");
  });

  it("BLOCKs if contract is paused", async () => {
    const res = await checkPaused({
      rpc: RPC,
      contract: CONTRACT,
      __client: mockPausedClient(true),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /PAUSED/);
  });

  it("ALLOWs if contract does not implement paused (RPC throw)", async () => {
    const res = await checkPaused({
      rpc: RPC,
      contract: CONTRACT,
      __client: mockPausedClient(new Error("method not found")),
    });
    // Safely defaults to ALLOW for non-pausable contracts
    assert.equal(res.decision, "ALLOW");
    assert.match(res.reason, /does not implement/);
  });

  it("BLOCKs on invalid input", async () => {
    assert.equal((await checkPaused({ rpc: RPC, contract: "0x123" })).decision, "BLOCK");
    assert.equal((await checkPaused({ rpc: "", contract: CONTRACT })).decision, "BLOCK");
  });
});
