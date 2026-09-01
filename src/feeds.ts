export const MAINNET_CHAIN_ID = 1;

export const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";

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
