import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { checkApproval, MAX_UINT256 } from "./allowance.js";

describe("checkApproval", () => {
  const TOKEN = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"; // USDC
  const SPENDER = "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45"; // Uniswap V3 Router

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
