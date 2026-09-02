#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/server";
import { StdioServerTransport } from "@modelcontextprotocol/server/stdio";
import { z } from "zod";
import { isStale } from "../isStale.js";
import { quoteFromFeed } from "../quote.js";
import { checkPrice } from "../check.js";

// Types for MCP — JSON cannot carry bigint, so we use string for answer/updatedAt
const server = new McpServer({
  name: "stale",
  version: "0.1.0",
});

// stale_isStale — pure, no RPC
server.registerTool(
  "stale_isStale",
  {
    description:
      "Check if a Chainlink price is stale. Pure, no RPC. Returns ALLOW/BLOCK with age and reason. Fail closed on missing/zero/negative/unparseable/future.",
    inputSchema: z.object({
      updatedAt: z
        .string()
        .describe("updatedAt timestamp as string (bigint seconds, e.g. '1724520000' or '0x...')"),
      nowSeconds: z
        .number()
        .int()
        .describe("Current time in seconds (e.g. Math.floor(Date.now()/1000))"),
      maxAgeSeconds: z
        .number()
        .int()
        .min(0)
        .describe("Max allowed age in seconds (policy, no default)"),
    }),
  },
  async ({
    updatedAt,
    nowSeconds,
    maxAgeSeconds,
  }: {
    updatedAt: string;
    nowSeconds: number;
    maxAgeSeconds: number;
  }) => {
    const result = isStale({ updatedAt, nowSeconds, maxAgeSeconds });
    return {
      content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
    };
  },
);

// stale_quote — price math, no RPC
server.registerTool(
  "stale_quote",
  {
    description:
      "Price math from Data Feed answer + decimals. No RPC. Returns priceUsd and quoteUsd. Fail closed on 0n/bad decimals/NaN.",
    inputSchema: z.object({
      answer: z.string().describe("answer as string bigint, e.g. '245377000000'"),
      decimals: z
        .number()
        .int()
        .min(0)
        .max(36)
        .describe("Feed decimals, from decimals() on-chain (never hardcoded 8)"),
      amountEth: z
        .number()
        .min(0)
        .nullable()
        .optional()
        .describe("Human ETH amount for quote, e.g. 0.5, or null"),
    }),
  },
  async ({
    answer,
    decimals,
    amountEth,
  }: {
    answer: string;
    decimals: number;
    amountEth?: number | null;
  }) => {
    try {
      const result = quoteFromFeed({
        answer: BigInt(answer),
        decimals,
        amountEth: amountEth ?? null,
      });
      return {
        content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
      };
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      return {
        content: [{ type: "text", text: `BLOCK — quote failed: ${msg}` }],
        isError: true,
      };
    }
  },
);

// stale_check — full check via viem, no wallet, no tx, fail closed
server.registerTool(
  "stale_check",
  {
    description:
      "Full guardrail: viem latestRoundData + decimals → isStale → quote. No wallet, no tx, fail closed. Returns same JSON as CLI --json with allowExecute. Default feed is ETH/USD mainnet; an optional feed must be allowlisted: ETH/USD 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419 or BTC/USD 0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c on Ethereum mainnet. Unknown feed → BLOCK.",
    inputSchema: z.object({
      rpc: z.string().describe("Ethereum RPC URL (e.g. https://ethereum-rpc.publicnode.com)"),
      feed: z
        .string()
        .describe("Data Feed proxy address, e.g. 0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"),
      maxAgeSeconds: z
        .number()
        .int()
        .min(0)
        .describe("Max allowed age in seconds (policy, no default)"),
      amountEth: z
        .number()
        .min(0)
        .nullable()
        .optional()
        .describe("Human ETH amount for quote, e.g. 0.5"),
      nowSeconds: z.number().int().optional().describe("Override now for testing, else Date.now()"),
    }),
  },
  async ({
    rpc,
    feed,
    maxAgeSeconds,
    amountEth,
    nowSeconds,
  }: {
    rpc: string;
    feed: string;
    maxAgeSeconds: number;
    amountEth?: number | null;
    nowSeconds?: number;
  }) => {
    const result = await checkPrice({
      rpc,
      feed,
      maxAgeSeconds,
      amountEth: amountEth ?? null,
      nowSeconds,
    });
    return {
      content: [{ type: "text", text: JSON.stringify(result, null, 2) }],
    };
  },
);

async function main(): Promise<void> {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((err: unknown) => {
  console.error(err instanceof Error ? err.message : String(err));
  process.exit(1);
});
