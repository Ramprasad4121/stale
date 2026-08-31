#!/usr/bin/env node
import { parseArgs } from "node:util";
import { checkPrice } from "./check.js";

const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"; // ETH/USD Ethereum mainnet — https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet

function truncate(s: string, n = 200): string {
  return s.length > n ? s.slice(0, n) : s;
}

function printHelp(): void {
  console.log(`stale — guardrail for onchain agents
Usage: stale --rpc <url> --maxAge <seconds> [--feed <address>] [--amount <eth>] [--json]

  --rpc     Ethereum RPC URL (required)
  --maxAge  Max allowed age in seconds (required, no default)
  --feed    Data Feed proxy address (default: ${DEFAULT_FEED} ETH/USD)
  --amount  Human ETH amount for quote, e.g. 0.5 (optional)
  --json    Output single JSON object only (default: human lines)
  --help    Show this help

Examples:
  stale --rpc https://ethereum-rpc.publicnode.com --maxAge 3600
  stale --rpc $RPC_URL --maxAge 60 --amount 0.5
  stale --rpc $RPC_URL --maxAge 60 --amount 0.5 --json
`);
}

async function main(): Promise<void> {
  // Node's official parseArgs (node:util) — type-safe, strict, no manual loop per https://nodejs.org/api/util.html#utilparseargsconfig
  const { values } = parseArgs({
    options: {
      rpc: { type: "string" },
      maxAge: { type: "string" },
      feed: { type: "string" },
      amount: { type: "string" },
      json: { type: "boolean", default: false },
      help: { type: "boolean", short: "h" },
    },
    strict: true,
    allowPositionals: false,
  }) as {
    values: {
      rpc?: string;
      maxAge?: string;
      feed?: string;
      amount?: string;
      json?: boolean;
      help?: boolean;
    };
  };

  if (values.help) {
    printHelp();
    process.exit(0);
  }

  if (!values.rpc) {
    console.log("BLOCK — missing --rpc — BLOCK (fail closed)");
    console.log(truncate("notify: provide --rpc <ethereum-rpc-url>"));
    printHelp();
    process.exit(1);
  }
  if (!values.maxAge) {
    console.log("BLOCK — missing --maxAge — BLOCK (fail closed)");
    console.log(truncate("notify: provide --maxAge <seconds> (no default)"));
    printHelp();
    process.exit(1);
  }

  const maxAgeSeconds = Number(values.maxAge);
  if (!Number.isFinite(maxAgeSeconds) || !Number.isInteger(maxAgeSeconds) || maxAgeSeconds < 0) {
    console.log(truncate(`BLOCK — invalid --maxAge "${values.maxAge}" — must be integer >= 0 — BLOCK`));
    process.exit(1);
  }

  const feed = values.feed ?? DEFAULT_FEED;
  if (!/^0x[a-fA-F0-9]{40}$/.test(feed)) {
    console.log(truncate(`BLOCK — invalid --feed "${feed}" — BLOCK`));
    process.exit(1);
  }

  let amountEth: number | null = null;
  if (values.amount !== undefined) {
    const raw = values.amount.trim();
    if (raw === "") {
      console.log(truncate(`BLOCK — invalid --amount "${values.amount}" — BLOCK (fail closed)`));
      process.exit(1);
    }
    const n = Number(raw);
    if (!Number.isFinite(n) || n < 0) {
      console.log(truncate(`BLOCK — invalid --amount "${values.amount}" — must be number >= 0 — BLOCK`));
      process.exit(1);
    }
    amountEth = n;
  }

  const result = await checkPrice({
    rpc: values.rpc,
    feed,
    maxAgeSeconds,
    amountEth,
  });

  if (values.json) {
    console.log(JSON.stringify(result));
    process.exit(result.allowExecute ? 0 : 1);
  }

  console.log(`${result.decision} — ${truncate(result.reason)}`);
  console.log(`feed=${result.feed} answer=${result.answer} updatedAt=${result.updatedAt} age=${result.ageSeconds ?? "null"}s maxAge=${result.maxAgeSeconds}s now=${result.now}`);
  console.log(`priceUsd=${result.priceUsd ?? "null"} amountEth=${result.amountEth ?? "null"} quoteUsd=${result.quoteUsd ?? "null"}`);

  if (!result.allowExecute) {
    console.log(truncate(`notify: ${result.reason} — do not act`));
    console.log(`execute: skipped`);
    process.exit(1);
  } else {
    console.log(`execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}`);
    process.exit(0);
  }
}

main().catch((err: unknown) => {
  const msg = err instanceof Error ? err.message : String(err);
  console.log(truncate(`BLOCK — unexpected error — BLOCK (fail closed)`));
  console.log(truncate(`notify: ${msg}`));
  process.exit(1);
});
