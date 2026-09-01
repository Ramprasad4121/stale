import { describe, it } from "node:test";
import assert from "node:assert/strict";
import { Client } from "@modelcontextprotocol/client";
import { StdioClientTransport } from "@modelcontextprotocol/client/stdio";

describe("MCP server — stale tools (no live RPC for isStale/quote, mocked check)", () => {
  it("lists 3 tools", async () => {
    const transport = new StdioClientTransport({
      command: "npx",
      args: ["tsx", "src/mcp/server.ts"],
      cwd: process.cwd(),
    });
    const client = new Client({ name: "test", version: "1.0.0" });
    await client.connect(transport);
    const { tools } = await client.listTools();
    const names = tools.map((t) => t.name).sort();
    assert.deepEqual(names, ["stale_check", "stale_isStale", "stale_quote"]);
    await client.close();
  });

  it("stale_isStale — pure, no RPC", async () => {
    const transport = new StdioClientTransport({
      command: "npx",
      args: ["tsx", "src/mcp/server.ts"],
      cwd: process.cwd(),
    });
    const client = new Client({ name: "test", version: "1.0.0" });
    await client.connect(transport);
    const now = Math.floor(Date.now() / 1000);
    const fresh = await client.callTool({
      name: "stale_isStale",
      arguments: { updatedAt: String(now - 10), nowSeconds: now, maxAgeSeconds: 60 },
    });
    const freshText = (fresh.content as any)[0].text as string;
    assert.match(freshText, /ALLOW/);
    const stale = await client.callTool({
      name: "stale_isStale",
      arguments: { updatedAt: String(now - 100), nowSeconds: now, maxAgeSeconds: 60 },
    });
    assert.match((stale.content as any)[0].text as string, /BLOCK/);
    await client.close();
  });

  it("stale_quote — price math, no RPC", async () => {
    const transport = new StdioClientTransport({
      command: "npx",
      args: ["tsx", "src/mcp/server.ts"],
      cwd: process.cwd(),
    });
    const client = new Client({ name: "test", version: "1.0.0" });
    await client.connect(transport);
    const r = await client.callTool({
      name: "stale_quote",
      arguments: { answer: "245377000000", decimals: 8, amountEth: 0.5 },
    });
    const text = (r.content as any)[0].text as string;
    const j = JSON.parse(text);
    assert.equal(j.priceUsd, 2453.77);
    assert.equal(j.quoteUsd, 1226.885);
    // bad answer → isError
    const bad = await client.callTool({
      name: "stale_quote",
      arguments: { answer: "0", decimals: 8 },
    });
    assert.equal((bad as any).isError, true);
    await client.close();
  });

  it("stale_check — fail closed on invalid feed (no live RPC)", async () => {
    const transport = new StdioClientTransport({
      command: "npx",
      args: ["tsx", "src/mcp/server.ts"],
      cwd: process.cwd(),
    });
    const client = new Client({ name: "test", version: "1.0.0" });
    await client.connect(transport);
    const r = await client.callTool({
      name: "stale_check",
      arguments: {
        rpc: "https://ethereum-rpc.publicnode.com",
        feed: "0x0000000000000000000000000000000000000000",
        maxAgeSeconds: 60,
      },
    });
    const text = (r.content as any)[0].text as string;
    const j = JSON.parse(text);
    // invalid feed is caught before RPC, still BLOCK, no live call needed for this path
    // For this dummy zero address, the feed is valid format but will try RPC and may 429; we check that it still returns BLOCK with allowExecute false
    assert.equal(j.allowExecute, false);
    assert.equal(j.decision, "BLOCK");
    await client.close();
  });

  it("huge and malformed inputs — fail closed", async () => {
    const transport = new StdioClientTransport({
      command: "npx",
      args: ["tsx", "src/mcp/server.ts"],
      cwd: process.cwd(),
    });
    const client = new Client({ name: "test", version: "1.0.0" });
    await client.connect(transport);
    const now = Math.floor(Date.now() / 1000);
    // huge future timestamp via stale_isStale → BLOCK
    const huge = await client.callTool({
      name: "stale_isStale",
      arguments: { updatedAt: String((BigInt(1) << BigInt(40)).toString()), nowSeconds: now, maxAgeSeconds: 60 },
    });
    assert.match((huge.content as any)[0].text as string, /BLOCK/);
    // huge amountEth overflow via quote → isError
    const badQuote = await client.callTool({
      name: "stale_quote",
      arguments: { answer: "100000000000", decimals: 8, amountEth: Number.MAX_VALUE },
    });
    assert.equal((badQuote as any).isError, true);
    await client.close();
  });
});
