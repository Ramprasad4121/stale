import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkIsContract } from "./contract.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const CONTRACT = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC
const EOA = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045"; // vitalik.eth

function mockContractClient(
  bytecode: string | undefined | Error,
): Pick<PublicClient, "getBytecode"> {
  return {
    getBytecode: async () => {
      if (bytecode instanceof Error) throw bytecode;
      return bytecode as `0x${string}` | undefined;
    },
  };
}

describe("checkIsContract", () => {
  it("ALLOWs if address has bytecode", async () => {
    const res = await checkIsContract({
      rpc: RPC,
      address: CONTRACT,
      __client: mockContractClient("0x60806040"),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs if address is an EOA (0x bytecode)", async () => {
    const res = await checkIsContract({
      rpc: RPC,
      address: EOA,
      __client: mockContractClient("0x"),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /PHISHING DANGER/);
  });

  it("BLOCKs if address is an EOA (undefined bytecode)", async () => {
    const res = await checkIsContract({
      rpc: RPC,
      address: EOA,
      __client: mockContractClient(undefined),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /PHISHING DANGER/);
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkIsContract({
      rpc: RPC,
      address: CONTRACT,
      __client: mockContractClient(new Error("RPC timeout")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to fetch bytecode/);
  });

  it("BLOCKs on invalid input", async () => {
    assert.equal((await checkIsContract({ rpc: RPC, address: "0x123" })).decision, "BLOCK");
    assert.equal((await checkIsContract({ rpc: "", address: CONTRACT })).decision, "BLOCK");
  });
});
