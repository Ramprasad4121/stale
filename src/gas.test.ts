import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkGasPrice } from "./gas.js";

const RPC = "https://ethereum-rpc.publicnode.com";

function mockClient(gasPriceWei: bigint | Error): Pick<PublicClient, "getGasPrice"> {
  return {
    getGasPrice: async () => {
      if (gasPriceWei instanceof Error) throw gasPriceWei;
      return gasPriceWei;
    },
  };
}

describe("checkGasPrice", () => {
  it("ALLOWs when gas is below threshold", async () => {
    // 20 Gwei network, 50 Gwei max
    const res = await checkGasPrice({
      rpc: RPC,
      maxGasPriceGwei: 50,
      __client: mockClient(20000000000n),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs when gas exceeds threshold", async () => {
    // 100 Gwei network, 50 Gwei max
    const res = await checkGasPrice({
      rpc: RPC,
      maxGasPriceGwei: 50,
      __client: mockClient(100000000000n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /network gas price 100 gwei exceeds maximum allowed 50 gwei/);
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkGasPrice({
      rpc: RPC,
      maxGasPriceGwei: 50,
      __client: mockClient(new Error("RPC timeout")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to fetch network gas price/);
  });

  it("BLOCKs on invalid maxGasPrice", async () => {
    const r1 = await checkGasPrice({ rpc: RPC, maxGasPriceGwei: 0, __client: mockClient(10n) });
    assert.equal(r1.decision, "BLOCK");
    assert.match(r1.reason, /maxGasPriceGwei must be > 0/);

    const r2 = await checkGasPrice({ rpc: RPC, maxGasPriceGwei: -5, __client: mockClient(10n) });
    assert.equal(r2.decision, "BLOCK");

    // @ts-expect-error
    const r3 = await checkGasPrice({ rpc: RPC, maxGasPriceGwei: "50", __client: mockClient(10n) });
    assert.equal(r3.decision, "BLOCK");
  });

  it("BLOCKs on missing rpc", async () => {
    const res = await checkGasPrice({ rpc: "", maxGasPriceGwei: 50, __client: mockClient(10n) });
    assert.equal(res.decision, "BLOCK");
  });
});
