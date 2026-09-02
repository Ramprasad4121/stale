import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkTokenTax } from "./honeypot.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const TOKEN = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const HOLDER = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";

function mockHoneypotClient(opts: {
  balanceOf?: bigint;
  simulateReverts?: boolean;
  simulateError?: Error;
}): Pick<PublicClient, "readContract" | "simulateContract"> {
  return {
    readContract: async () => opts.balanceOf ?? 0n,
    simulateContract: async () => {
      if (opts.simulateReverts) {
        throw new Error("execution reverted: transfer blocked");
      }
      if (opts.simulateError) {
        throw opts.simulateError;
      }
      return { result: true };
    },
  } as unknown as Pick<PublicClient, "readContract" | "simulateContract">;
}

describe("checkTokenTax", () => {
  it("ALLOWs if transfer simulation succeeds (legitimate token)", async () => {
    const res = await checkTokenTax({
      rpc: RPC,
      token: TOKEN,
      holder: HOLDER,
      amount: 1000000n,
      __client: mockHoneypotClient({ balanceOf: 500000n }),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs if transfer reverts (honeypot)", async () => {
    const res = await checkTokenTax({
      rpc: RPC,
      token: TOKEN,
      holder: HOLDER,
      amount: 1000000n,
      __client: mockHoneypotClient({ simulateReverts: true }),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /HONEYPOT DETECTED/);
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkTokenTax({
      rpc: RPC,
      token: TOKEN,
      holder: HOLDER,
      amount: 1000000n,
      __client: mockHoneypotClient({ simulateError: new Error("rpc timeout") }),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to simulate token transfer/);
  });

  it("BLOCKs on invalid inputs", async () => {
    assert.equal(
      (await checkTokenTax({ rpc: RPC, token: "0x123", holder: HOLDER, amount: 100n })).decision,
      "BLOCK",
    );
    assert.equal(
      (await checkTokenTax({ rpc: RPC, token: TOKEN, holder: "0x123", amount: 100n })).decision,
      "BLOCK",
    );
    assert.equal(
      (await checkTokenTax({ rpc: RPC, token: TOKEN, holder: HOLDER, amount: -1n })).decision,
      "BLOCK",
    );
    assert.equal(
      (await checkTokenTax({ rpc: "", token: TOKEN, holder: HOLDER, amount: 100n })).decision,
      "BLOCK",
    );
    assert.equal(
      (
        await checkTokenTax({
          rpc: RPC,
          token: TOKEN,
          holder: HOLDER,
          amount: 100n,
          maxTaxPercent: -1,
        })
      ).decision,
      "BLOCK",
    );
  });
});
