#!/usr/bin/env node
import { createPublicClient, http, parseAbi, formatUnits } from "viem";
import { mainnet } from "viem/chains";
import { isStale } from "./isStale.js";

const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"; // ETH/USD Ethereum mainnet — https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet

const feedAbi = parseAbi([
  "function decimals() view returns (uint8)",
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
]);

type Args = {
  rpc?: string;
  maxAge?: string;
  feed?: string;
  amount?: string;
  json?: boolean;
  help?: boolean;
};

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
    console.log("notify: provide --rpc <ethereum-rpc-url>");
    printHelp();
    process.exit(1);
  }
  if (!args.maxAge) {
    console.log("BLOCK — missing --maxAge — BLOCK (fail closed)");
    console.log("notify: provide --maxAge <seconds> (no default)");
    printHelp();
    process.exit(1);
  }

  const maxAgeSeconds = Number(args.maxAge);
  if (!Number.isFinite(maxAgeSeconds) || maxAgeSeconds < 0 || !Number.isInteger(maxAgeSeconds)) {
    console.log(`BLOCK — invalid --maxAge "${args.maxAge}" — must be integer >= 0 — BLOCK`);
    process.exit(1);
  }

  const feed = args.feed ?? DEFAULT_FEED;
  if (!/^0x[a-fA-F0-9]{40}$/.test(feed)) {
    console.log(`BLOCK — invalid --feed "${feed}" — BLOCK`);
    process.exit(1);
  }

  // Fail closed: bad --amount → BLOCK
  let amountEth: number | null = null;
  if (args.amount !== undefined) {
    const raw = args.amount.trim();
    if (raw === "") {
      console.log(`BLOCK — invalid --amount "${args.amount}" — BLOCK (fail closed)`);
      process.exit(1);
    }
    const n = Number(raw);
    if (!Number.isFinite(n) || n < 0) {
      console.log(`BLOCK — invalid --amount "${args.amount}" — must be number >= 0 — BLOCK`);
      process.exit(1);
    }
    amountEth = n;
  }

  const client = createPublicClient({
    chain: mainnet,
    transport: http(args.rpc),
  });

  let answer: bigint;
  let updatedAt: bigint;
  try {
    const data = (await client.readContract({
      address: feed as `0x${string}`,
      abi: feedAbi,
      functionName: "latestRoundData",
    })) as readonly [bigint, bigint, bigint, bigint, bigint];
    // latestRoundData returns (roundId, answer, startedAt, updatedAt, answeredInRound)
    answer = data[1];
    updatedAt = data[3];
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.log(`BLOCK — failed to read latestRoundData from ${feed} — BLOCK (fail closed)`);
    console.log(`notify: ${msg}`);
    process.exit(1);
  }

  // Fail closed if contract returned zero/missing timestamp or answer is unusable
  // (updatedAt == 0 means no data yet on that proxy)
  if (updatedAt === 0n) {
    console.log(`BLOCK — updatedAt is 0 (no data) from ${feed} — BLOCK`);
    console.log(`notify: feed returned updatedAt=0, answer=${answer.toString()}`);
    process.exit(1);
  }
  if (answer <= 0n) {
    console.log(`BLOCK — answer is ${answer.toString()} (invalid price) from ${feed} — BLOCK`);
    console.log(`notify: feed returned answer=${answer.toString()}, updatedAt=${updatedAt.toString()}`);
    process.exit(1);
  }

  // Fetch decimals from feed — do not hardcode 8. Fail closed if call fails.
  let decimals: number;
  try {
    const d = (await client.readContract({
      address: feed as `0x${string}`,
      abi: feedAbi,
      functionName: "decimals",
    })) as number | bigint;
    decimals = typeof d === "bigint" ? Number(d) : d;
    if (!Number.isInteger(decimals) || decimals < 0 || decimals > 36) {
      throw new Error(`invalid decimals ${String(d)}`);
    }
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    console.log(`BLOCK — failed to read decimals() from ${feed} — BLOCK (fail closed)`);
    console.log(`notify: ${msg}`);
    process.exit(1);
  }

  const nowSeconds = Math.floor(Date.now() / 1000);
  const result = isStale({ updatedAt, nowSeconds, maxAgeSeconds });

  // Quote block — official answer scaled by feed decimals
  const priceUsd = Number(formatUnits(answer, decimals));
  const quoteUsd = amountEth !== null ? amountEth * priceUsd : null;

  const allowExecute = result.decision === "ALLOW";
  const payload = {
    decision: result.decision,
    reason: result.reason,
    feed,
    answer: answer.toString(),
    priceUsd,
    amountEth,
    quoteUsd,
    updatedAt: updatedAt.toString(),
    ageSeconds: result.ageSeconds,
    maxAgeSeconds,
    now: nowSeconds,
    allowExecute,
  };

  // --json prints only JSON. Default is human lines only.
  if (args.json) {
    console.log(JSON.stringify(payload));
    process.exit(allowExecute ? 0 : 1);
  }

  // Human output
  console.log(`${result.decision} — ${result.reason}`);
  console.log(`feed=${feed} answer=${answer.toString()} updatedAt=${updatedAt.toString()} age=${result.ageSeconds ?? "null"}s maxAge=${maxAgeSeconds}s now=${nowSeconds}`);
  console.log(`priceUsd=${priceUsd} amountEth=${amountEth ?? "null"} quoteUsd=${quoteUsd ?? "null"} (decimals=${decimals})`);

  if (!allowExecute) {
    console.log(`notify: price is stale or not-yet-valid — do not act`);
    console.log(`execute: skipped`);
    process.exit(1);
  } else {
    console.log(`execute: {"action":"none","note":"v3 dry-run. no tx. no Agents call."}`);
    process.exit(0);
  }
}

main().catch((err) => {
  const msg = err instanceof Error ? err.message : String(err);
  console.log(`BLOCK — unexpected error — BLOCK (fail closed)`);
  console.log(`notify: ${msg}`);
  process.exit(1);
});
