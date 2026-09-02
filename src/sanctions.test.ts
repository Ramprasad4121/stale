import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkSanctioned } from "./sanctions.js";

const RPC = "https://ethereum-rpc.publicnode.com";
const CLEAN_ADDRESS = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
const TORNADO_ROUTER = "0xd90e2f925DA726b50C4Ed8D0Fb90Ad053324F31b";

function mockSanctionsClient(isSanctioned: boolean | Error): Pick<PublicClient, "readContract"> {
  return {
    readContract: (async () => {
      if (isSanctioned instanceof Error) throw isSanctioned;
      return isSanctioned;
    }) as unknown as PublicClient["readContract"],
  };
}

describe("checkSanctioned", () => {
  it("ALLOWs if address is clean", async () => {
    const res = await checkSanctioned({
      rpc: RPC,
      address: CLEAN_ADDRESS,
      __client: mockSanctionsClient(false),
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs if address is sanctioned (Tornado Cash etc)", async () => {
    const res = await checkSanctioned({
      rpc: RPC,
      address: TORNADO_ROUTER,
      __client: mockSanctionsClient(true),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /COMPLIANCE VIOLATION/);
  });

  it("BLOCKs on RPC error (fail closed)", async () => {
    const res = await checkSanctioned({
      rpc: RPC,
      address: CLEAN_ADDRESS,
      __client: mockSanctionsClient(new Error("RPC timeout")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to query sanctions oracle/);
  });

  it("BLOCKs on invalid input", async () => {
    assert.equal((await checkSanctioned({ rpc: RPC, address: "0x123" })).decision, "BLOCK");
    assert.equal((await checkSanctioned({ rpc: "", address: CLEAN_ADDRESS })).decision, "BLOCK");
  });
});
