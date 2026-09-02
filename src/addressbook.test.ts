import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { AddressBook } from "./addressbook.js";

const USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const WETH = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2";
const UNKNOWN = "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045";

describe("AddressBook", () => {
  it("ALLOWs allowlisted addresses", () => {
    const book = new AddressBook({
      allowlist: { USDC: USDC, WETH: WETH },
    });
    assert.equal(book.check(USDC).decision, "ALLOW");
    assert.equal(book.check(WETH).decision, "ALLOW");
    assert.match(book.check(USDC).reason, /allowlisted as "USDC"/);
  });

  it("BLOCKs unknown addresses in strict mode", () => {
    const book = new AddressBook({
      allowlist: { USDC: USDC },
      strict: true,
    });
    const res = book.check(UNKNOWN);
    assert.equal(res.decision, "BLOCK");
    assert.match(res.reason, /UNKNOWN ADDRESS/);
  });

  it("ALLOWs unknown addresses when strict is off", () => {
    const book = new AddressBook({
      allowlist: { USDC: USDC },
      strict: false,
    });
    assert.equal(book.check(UNKNOWN).decision, "ALLOW");
  });

  it("is case-insensitive", () => {
    const book = new AddressBook({ allowlist: { USDC: USDC } });
    assert.equal(book.check(USDC.toLowerCase()).decision, "ALLOW");
    assert.equal(book.check(USDC.toUpperCase().replace("X", "x")).decision, "ALLOW");
  });

  it("has(), labelOf(), size() work", () => {
    const book = new AddressBook({ allowlist: { USDC: USDC, WETH: WETH } });
    assert.equal(book.has(USDC), true);
    assert.equal(book.has(UNKNOWN), false);
    assert.equal(book.labelOf(USDC), "USDC");
    assert.equal(book.size(), 2);
  });

  it("throws on invalid address in config", () => {
    assert.throws(() => new AddressBook({ allowlist: { bad: "0x123" } }));
  });

  it("BLOCKs on invalid address input", () => {
    const book = new AddressBook({ allowlist: { USDC: USDC } });
    assert.equal(book.check("0x123").decision, "BLOCK");
  });
});
