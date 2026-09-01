export const MAINNET_CHAIN_ID = 1;

export const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";

// Official proxies — https://docs.chain.link/data-feeds/price-feeds/addresses?network=ethereum
export const BTC_USD_FEED = "0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c";

export type FeedEntry = {
  address: string;
  symbol: string;
  chainId: number;
};

const registry: FeedEntry[] = [
  {
    address: DEFAULT_FEED,
    symbol: "ETH/USD",
    chainId: MAINNET_CHAIN_ID,
  },
  {
    address: BTC_USD_FEED,
    symbol: "BTC/USD",
    chainId: MAINNET_CHAIN_ID,
  },
];

/**
 * Case-insensitive lookup. Unknown address → null (fail closed).
 */
export function lookupFeed(address: string): FeedEntry | null {
  const normalized = address.toLowerCase();
  for (const entry of registry) {
    if (entry.address.toLowerCase() === normalized) return entry;
  }
  return null;
}

export const FEEDS = registry;
