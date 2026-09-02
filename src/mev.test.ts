import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { checkMevRpc } from "./mev.js";

describe("checkMevRpc", () => {
  it("ALLOWs known MEV-protected RPCs", () => {
    assert.equal(checkMevRpc({ rpc: "https://rpc.flashbots.net" }).decision, "ALLOW");
    assert.equal(checkMevRpc({ rpc: "https://rpc.mevblocker.io/fast" }).decision, "ALLOW");
    assert.equal(checkMevRpc({ rpc: "https://rpc.beaverbuild.org" }).decision, "ALLOW");
  });

  it("BLOCKs public RPCs", () => {
    const res = checkMevRpc({ rpc: "https://eth-mainnet.g.alchemy.com/v2/KEY" });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /PUBLIC MEMPOOL DANGER/);

    assert.equal(checkMevRpc({ rpc: "https://mainnet.infura.io/v3/KEY" }).decision, "BLOCK");
    assert.equal(checkMevRpc({ rpc: "https://cloudflare-eth.com" }).decision, "BLOCK");
  });

  it("BLOCKs invalid URLs", () => {
    assert.equal(checkMevRpc({ rpc: "not a url" }).decision, "BLOCK");
    assert.equal(checkMevRpc({ rpc: "" }).decision, "BLOCK");
    // @ts-expect-error
    assert.equal(checkMevRpc({ rpc: null }).decision, "BLOCK");
  });
});
