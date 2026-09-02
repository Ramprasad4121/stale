import { describe, it } from "node:test";
import assert from "node:assert/strict";
import type { PublicClient } from "viem";
import { checkApproval, checkAllowance, MAX_UINT256 } from "./allowance.js";

const TOKEN = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC
const SPENDER = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"; // Uniswap V3 Router
const OWNER = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";
const RPC = "https://ethereum-rpc.publicnode.com";

function mockAllowanceClient(bal: bigint | Error): Pick<PublicClient, "readContract"> {
  return {
    readContract: (async ({ functionName }: { functionName: string }) => {
      if (functionName === "allowance") {
        if (bal instanceof Error) throw bal;
        return bal;
      }
      throw new Error("unexpected");
    }) as unknown as PublicClient["readContract"],
  };
}

describe("checkApproval", () => {
  it("ALLOWs an exact amount approval", () => {
    const res = checkApproval({
      token: TOKEN,
      spender: SPENDER,
      amount: 1000000000n, // 1000 USDC
    });
    assert.equal(res.decision, "ALLOW");
    assert.equal(res.allowExecute, true);
  });

  it("BLOCKs infinite approval (MaxUint256)", () => {
    const res = checkApproval({
      token: TOKEN,
      spender: SPENDER,
      amount: MAX_UINT256,
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /infinite approval/);
  });

  it("BLOCKs dangerously large approvals (> 2^255)", () => {
    const huge = 2n ** 255n + 1n;
    const res = checkApproval({
      token: TOKEN,
      spender: SPENDER,
      amount: huge,
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /dangerously large approval/);
  });

  it("BLOCKs invalid addresses", () => {
    const res = checkApproval({
      token: "0x123",
      spender: SPENDER,
      amount: 100n,
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /invalid token address/);

    const res2 = checkApproval({
      token: TOKEN,
      spender: "0xinvalid",
      amount: 100n,
    });
    assert.equal(res2.decision, "BLOCK");
    assert.match(res2.reason, /invalid spender address/);
  });

  it("BLOCKs invalid types or negatives", () => {
    // @ts-expect-error
    const r1 = checkApproval({ token: TOKEN, spender: SPENDER, amount: -1n });
    assert.equal(r1.decision, "BLOCK");

    // @ts-expect-error
    const r2 = checkApproval({ token: TOKEN, spender: SPENDER, amount: 100 });
    assert.equal(r2.decision, "BLOCK");
  });
});

describe("checkAllowance", () => {
  it("ALLOWs when allowance >= required", async () => {
    const res = await checkAllowance({
      rpc: RPC,
      token: TOKEN,
      owner: OWNER,
      spender: SPENDER,
      requiredAmount: 100n,
      __client: mockAllowanceClient(150n),
    });
    assert.equal(res.decision, "ALLOW");
  });

  it("BLOCKs when allowance < required", async () => {
    const res = await checkAllowance({
      rpc: RPC,
      token: TOKEN,
      owner: OWNER,
      spender: SPENDER,
      requiredAmount: 100n,
      __client: mockAllowanceClient(50n),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /insufficient allowance/);
  });

  it("BLOCKs on RPC error", async () => {
    const res = await checkAllowance({
      rpc: RPC,
      token: TOKEN,
      owner: OWNER,
      spender: SPENDER,
      requiredAmount: 100n,
      __client: mockAllowanceClient(new Error("rpc error")),
    });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /failed to read allowance/);
  });

  it("BLOCKs on invalid inputs", async () => {
    assert.equal(
      (
        await checkAllowance({
          rpc: RPC,
          token: "0x123",
          owner: OWNER,
          spender: SPENDER,
          requiredAmount: 100n,
        })
      ).decision,
      "BLOCK",
    );
    assert.equal(
      (
        await checkAllowance({
          rpc: RPC,
          token: TOKEN,
          owner: "0x123",
          spender: SPENDER,
          requiredAmount: 100n,
        })
      ).decision,
      "BLOCK",
    );
    assert.equal(
      (
        await checkAllowance({
          rpc: RPC,
          token: TOKEN,
          owner: OWNER,
          spender: "0x123",
          requiredAmount: 100n,
        })
      ).decision,
      "BLOCK",
    );
    // @ts-expect-error
    assert.equal(
      (
        await checkAllowance({
          rpc: RPC,
          token: TOKEN,
          owner: OWNER,
          spender: SPENDER,
          requiredAmount: -1n,
        })
      ).decision,
      "BLOCK",
    );
    assert.equal(
      (
        await checkAllowance({
          rpc: "",
          token: TOKEN,
          owner: OWNER,
          spender: SPENDER,
          requiredAmount: 100n,
        })
      ).decision,
      "BLOCK",
    );
  });
});
