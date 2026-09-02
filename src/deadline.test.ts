import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { checkDeadline } from "./deadline.js";

describe("checkDeadline", () => {
  const now = Math.floor(Date.now() / 1000);

  it("ALLOWs a reasonable deadline", () => {
    const res = checkDeadline({ deadline: now + 300 }); // 5 min from now
    assert.equal(res.decision, "ALLOW");
  });

  it("BLOCKs expired deadline", () => {
    const res = checkDeadline({ deadline: now - 60 });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /EXPIRED DEADLINE/);
  });

  it("BLOCKs deadline too tight", () => {
    const res = checkDeadline({ deadline: now + 5, minFutureSeconds: 30 });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /DEADLINE TOO TIGHT/);
  });

  it("BLOCKs deadline too far in future", () => {
    const res = checkDeadline({ deadline: now + 7200, maxFutureSeconds: 1200 });
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /DEADLINE TOO FAR/);
  });

  it("BLOCKs on invalid input", () => {
    assert.equal(checkDeadline({ deadline: -1 }).decision, "BLOCK");
    assert.equal(checkDeadline({ deadline: NaN }).decision, "BLOCK");
    assert.equal(checkDeadline({ deadline: now + 300, maxFutureSeconds: -1 }).decision, "BLOCK");
    assert.equal(checkDeadline({ deadline: now + 300, minFutureSeconds: -1 }).decision, "BLOCK");
  });
});
