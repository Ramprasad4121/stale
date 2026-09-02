import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { AuditLogger } from "./audit.js";

describe("AuditLogger", () => {
  it("records and retrieves entries", () => {
    const logger = new AuditLogger();
    logger.record("checkPrice", "ALLOW", "price is fresh");
    logger.record("checkGas", "BLOCK", "gas too high", { gasPrice: "100gwei" });

    assert.equal(logger.size(), 2);
    const entries = logger.getEntries();
    assert.equal(entries[0].guardrail, "checkPrice");
    assert.equal(entries[0].decision, "ALLOW");
    assert.equal(entries[1].decision, "BLOCK");
    assert.equal(entries[1].metadata?.gasPrice, "100gwei");
  });

  it("getBlocks() filters correctly", () => {
    const logger = new AuditLogger();
    logger.record("a", "ALLOW", "ok");
    logger.record("b", "BLOCK", "bad");
    logger.record("c", "ALLOW", "ok");
    logger.record("d", "BLOCK", "worse");

    const blocks = logger.getBlocks();
    assert.equal(blocks.length, 2);
    assert.equal(blocks[0].guardrail, "b");
    assert.equal(blocks[1].guardrail, "d");
  });

  it("getByGuardrail() filters correctly", () => {
    const logger = new AuditLogger();
    logger.record("checkPrice", "ALLOW", "ok");
    logger.record("checkGas", "BLOCK", "bad");
    logger.record("checkPrice", "BLOCK", "stale");

    assert.equal(logger.getByGuardrail("checkPrice").length, 2);
    assert.equal(logger.getByGuardrail("checkGas").length, 1);
  });

  it("enforces maxEntries FIFO eviction", () => {
    const logger = new AuditLogger({ maxEntries: 3 });
    logger.record("a", "ALLOW", "1");
    logger.record("b", "ALLOW", "2");
    logger.record("c", "ALLOW", "3");
    logger.record("d", "ALLOW", "4"); // evicts 'a'

    assert.equal(logger.size(), 3);
    assert.equal(logger.getEntries()[0].guardrail, "b");
  });

  it("fires onEntry callback", () => {
    const captured: string[] = [];
    const logger = new AuditLogger({
      onEntry: (entry) => captured.push(entry.guardrail),
    });
    logger.record("x", "ALLOW", "ok");
    logger.record("y", "BLOCK", "bad");
    assert.deepEqual(captured, ["x", "y"]);
  });

  it("survives onEntry callback throwing", () => {
    const logger = new AuditLogger({
      onEntry: () => {
        throw new Error("boom");
      },
    });
    // Should not throw
    logger.record("x", "ALLOW", "ok");
    assert.equal(logger.size(), 1);
  });

  it("clear() and toJSON() work", () => {
    const logger = new AuditLogger();
    logger.record("x", "ALLOW", "ok");
    assert.equal(logger.size(), 1);

    const json = logger.toJSON();
    assert.ok(json.includes("x"));

    logger.clear();
    assert.equal(logger.size(), 0);
  });
});
