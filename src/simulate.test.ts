import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { simulateTx } from "./simulate.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const ACCOUNT = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
const TO = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";

function mockSimulateClient(shouldRevert: boolean | Error): Pick<PublicClient, "call"> {
  return {
    call: async () => {
      if (shouldRevert === true)
        throw new Error("execution reverted: transfer amount exceeds balance");
      if (shouldRevert instanceof Error) throw shouldRevert;
      return { data: "0x" };
    },
  };
}

describe("simulateTx", () => {
  it("ALLOWs if simulation succeeds", async () => {
    const res = await simulateTx({
      rpc: RPC,
      account: ACCOUNT,
      to: TO,
      __client: mockSimulateClient(false),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs if simulation reverts", async () => {
    const res = await simulateTx({
      rpc: RPC,
      account: ACCOUNT,
      to: TO,
      data: "0xa9059cbb",
      value: 100n,
      __client: mockSimulateClient(true),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /execution reverted: transfer amount exceeds balance/);
  });

  it("BLOCKs on RPC/Network error", async () => {
    const res = await simulateTx({
      rpc: RPC,
      account: ACCOUNT,
      to: TO,
      __client: mockSimulateClient(new Error("network timeout")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /network timeout/);
  });

  it("BLOCKs on invalid input", async () => {
    assert.equal((await simulateTx({ rpc: RPC, account: "0x123", to: TO })).decision, "BLOCK");
    assert.equal((await simulateTx({ rpc: RPC, account: ACCOUNT, to: "0x123" })).decision, "BLOCK");
    assert.equal(
      (await simulateTx({ rpc: RPC, account: ACCOUNT, to: TO, data: "0xZZZ" })).decision,
      "BLOCK",
    );
    // @ts-expect-error
    assert.equal(
      (await simulateTx({ rpc: RPC, account: ACCOUNT, to: TO, value: -1n })).decision,
      "BLOCK",
    );
    assert.equal((await simulateTx({ rpc: "", account: ACCOUNT, to: TO })).decision, "BLOCK");
  });
});
