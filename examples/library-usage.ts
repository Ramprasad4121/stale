import { checkPrice, BTC_USD_FEED } from "../src/index.js";

/**
 * Example: Using `stale` as a Guardrail for On-Chain Agents
 *
 * This script demonstrates how an autonomous on-chain agent should verify
 * that Chainlink oracle data is fresh before executing a high-value trade.
 */
async function main() {
  // A public RPC endpoint (for mainnet)
  // Note: For production use, replace with your dedicated RPC (Alchemy, Infura, etc.)
  const RPC_URL = "https://cloudflare-eth.com";

  // The amount of ETH the agent intends to trade
  const tradeAmountEth = 0.5;

  console.log("Evaluating Chainlink feed freshness before trade execution...");

  // checkPrice evaluates the Chainlink feed and returns an ALLOW or BLOCK decision.
  // Design philosophy: FAIL-CLOSED.
  // Any RPC failure, missing data, invalid values (like 0 or negative prices),
  // or a price older than maxAgeSeconds will automatically return a BLOCK decision.
  const result = await checkPrice({
    rpc: RPC_URL,
    feed: BTC_USD_FEED, // Mainnet BTC/USD feed address
    maxAgeSeconds: 3600, // Reject if the last update is older than 1 hour (3600s)
    amountEth: tradeAmountEth,
  });

  if (result.decision === "BLOCK") {
    // Fail-Closed Action: The agent MUST abort the trade.
    // Proceeding could mean executing trades at wildly inaccurate prices,
    // leading to severe financial loss or exploitation.
    console.error(`🚨 Trade Blocked! Reason: ${result.reason}`);
    console.error(`Fail-closed safeguard triggered. Aborting execution.`);
    process.exit(1);
  }

  // If decision is ALLOW, the price is fresh, valid, and within the maxAgeSeconds bound.
  console.log(`✅ Price is fresh! (Updated ${result.ageSeconds} seconds ago)`);
  console.log(`💵 Current BTC Price: $${result.priceUsd}`);
  if (result.quoteUsd) {
    console.log(`💱 Quoted Value: $${result.quoteUsd}`);
  }

  console.log("🚀 Safety checks passed. Proceeding with on-chain trade execution...");
  // TODO: Implement your agent's transaction execution logic here...
}

main().catch(console.error);
