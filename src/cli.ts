#!/usr/bin/env node
import { checkPrice } from "./check.js";

const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"; // ETH/USD Ethereum mainnet — https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet

type Args = {
  rpc?: string;
  maxAge?: string;
  feed?: string;
  amount?: string;
  json?: boolean;
  help?: boolean;
};

function truncate(s: string, n = 200): string {
  return s.length > n ? s.slice(0, n) : s;
}

function parseArgs(argv: string[]): Args {
  const out: Args = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--rpc" && argv[i + 1]) out.rpc = argv[++i];
    else if (a === "--maxAge" && argv[i + 1]) out.maxAge = argv[++i];
    else if (a === "--feed" && argv[i + 1]) out.feed = argv[++i];
    else if (a === "--amount" && argv[i + 1]) out.amount = argv[++i];
    else if (a === "--json") out.json = true;
    else if (a === "--help" || a === "-h") out.help = true;
    else if (a.startsWith("--rpc=")) out.rpc = a.split("=")[1];
    else if (a.startsWith("--maxAge=")) out.maxAge = a.split("=")[1];
    else if (a.startsWith("--feed=")) out.feed = a.split("=")[1];
    else if (a.startsWith("--amount=")) out.amount = a.split("=")[1];
    else if (a === "--json=true" || a === "--json=1") out.json = true;
  }
  return out;
}

function printHelp() {
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

async function main() {
  const args = parseArgs(process.argv.slice(2));

  if (args.help) {
    printHelp();
    process.exit(0);
  }

  // Fail closed on missing args
  if (!args.rpc) {
    console.log("BLOCK — missing --rpc — BLOCK (fail closed)");
    console.log(truncate("notify: provide --rpc <ethereum-rpc-url>"));
    printHelp();
    process.exit(1);
  }
  if (!args.maxAge) {
    console.log("BLOCK — missing --maxAge — BLOCK (fail closed)");
    console.log(truncate("notify: provide --maxAge <seconds> (no default)"));
    printHelp();
    process.exit(1);
  }

  const maxAgeSeconds = Number(args.maxAge);
  if (!Number.isFinite(maxAgeSeconds) || maxAgeSeconds < 0 || !Number.isInteger(maxAgeSeconds)) {
    console.log(truncate(`BLOCK — invalid --maxAge "${args.maxAge}" — must be integer >= 0 — BLOCK`));
    process.exit(1);
  }

  const feed = args.feed ?? DEFAULT_FEED;
  if (!/^0x[a-fA-F0-9]{40}$/.test(feed)) {
    console.log(truncate(`BLOCK — invalid --feed "${feed}" — BLOCK`));
    process.exit(1);
  }

  let amountEth: number | null = null;
  if (args.amount !== undefined) {
    const raw = args.amount.trim();
    if (raw === "") {
      console.log(truncate(`BLOCK — invalid --amount "${args.amount}" — BLOCK (fail closed)`));
      process.exit(1);
    }
    const n = Number(raw);
    if (!Number.isFinite(n) || n < 0) {
      console.log(truncate(`BLOCK — invalid --amount "${args.amount}" — must be number >= 0 — BLOCK`));
      process.exit(1);
    }
    amountEth = n;
  }

  // checkPrice does all viem calls + isStale + quote, fail closed, no wallet, no write
  const result = await checkPrice({
    rpc: args.rpc,
    feed,
    maxAgeSeconds,
    amountEth,
  });

  // --json prints only JSON. Default is human lines only.
  if (args.json) {
    console.log(JSON.stringify(result));
    process.exit(result.allowExecute ? 0 : 1);
  }

  // Human output
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

main().catch((err) => {
  const msg = err instanceof Error ? err.message : String(err);
  console.log(truncate(`BLOCK — unexpected error — BLOCK (fail closed)`));
  console.log(truncate(`notify: ${msg}`));
  process.exit(1);
});
