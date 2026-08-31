#!/usr/bin/env node
import { createPublicClient, http, parseAbi } from "viem";
import { mainnet } from "viem/chains";
import { isStale } from "./isStale.js";

const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"; // ETH/USD Ethereum mainnet — https://docs.chain.link/data-feeds/price-feeds/addresses#ethereum-mainnet

const feedAbi = parseAbi([
  "function latestRoundData() view returns (uint80 roundId, int256 answer, uint256 startedAt, uint256 updatedAt, uint80 answeredInRound)",
]);

type Args = {
  rpc?: string;
  maxAge?: string;
  feed?: string;
  help?: boolean;
};

function parseArgs(argv: string[]): Args {
  const out: Args = {};
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--rpc" && argv[i + 1]) out.rpc = argv[++i];
    else if (a === "--maxAge" && argv[i + 1]) out.maxAge = argv[++i];
    else if (a === "--feed" && argv[i + 1]) out.feed = argv[++i];
    else if (a === "--help" || a === "-h") out.help = true;
    else if (a.startsWith("--rpc=")) out.rpc = a.split("=")[1];
    else if (a.startsWith("--maxAge=")) out.maxAge = a.split("=")[1];
    else if (a.startsWith("--feed=")) out.feed = a.split("=")[1];
  }
  return out;
}

function printHelp() {
  console.log(`stale — guardrail for onchain agents
Usage: stale --rpc <url> --maxAge <seconds> [--feed <address>]

  --rpc     Ethereum RPC URL (required)
  --maxAge  Max allowed age in seconds (required, no default)
  --feed    Data Feed proxy address (default: ${DEFAULT_FEED} ETH/USD)
  --help    Show this help

Examples:
  stale --rpc https://eth.llamarpc.com --maxAge 3600
  stale --rpc $RPC_URL --maxAge 60 --feed ${DEFAULT_FEED}
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

  const nowSeconds = Math.floor(Date.now() / 1000);
  const result = isStale({ updatedAt, nowSeconds, maxAgeSeconds });

  // Human + machine readable: first word is decision
  console.log(`${result.decision} — ${result.reason}`);
  console.log(`feed=${feed} answer=${answer.toString()} updatedAt=${updatedAt.toString()} age=${result.ageSeconds ?? "null"}s maxAge=${maxAgeSeconds}s now=${nowSeconds}`);

  if (result.decision === "BLOCK") {
    console.log(`notify: price is stale or not-yet-valid — do not act`);
    process.exit(1);
  } else {
    process.exit(0);
  }
}

main().catch((err) => {
  const msg = err instanceof Error ? err.message : String(err);
  console.log(`BLOCK — unexpected error — BLOCK (fail closed)`);
  console.log(`notify: ${msg}`);
  process.exit(1);
});
