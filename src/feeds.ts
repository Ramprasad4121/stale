/**
 * @module feeds
 * Contains the allowlist of officially supported Chainlink Data Feeds and L2 Sequencer feeds.
 * Stale strictly defaults to fail-closed if an unknown feed address is requested.
 */

export const MAINNET_CHAIN_ID = 1;
export const OPTIMISM_CHAIN_ID = 10;
export const POLYGON_CHAIN_ID = 137;
export const BASE_CHAIN_ID = 8453;
export const ARBITRUM_CHAIN_ID = 42161;

export const DEFAULT_FEED = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";

// Official proxies — https://docs.chain.link/data-feeds/price-feeds/addresses?network=ethereum
export const BTC_USD_FEED = "0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c";
export const USDC_USD_FEED = "0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6";

export const SEQUENCER_FEEDS: Record<number, string> = {
  [ARBITRUM_CHAIN_ID]: "0xFdB631F5EE196F0ed6FAa767959853A9F217697D",
  [OPTIMISM_CHAIN_ID]: "0x371EAD81c9102C9BF4874A9075FFFf170F2Ee389",
  [BASE_CHAIN_ID]: "0xBCF85224fc0756B9Fa45aA7892530B47e10b6433",
};

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
  {
    address: USDC_USD_FEED,
    symbol: "USDC/USD",
    chainId: MAINNET_CHAIN_ID,
  },
  {
    address: "0x13e3Ee699D1909E989722E753853AE30b17e08c5",
    symbol: "ETH/USD",
    chainId: OPTIMISM_CHAIN_ID,
  },
  {
    address: "0x718A5788b89454aAE3A028AE9c111A29Be6c2aB0",
    symbol: "BTC/USD",
    chainId: OPTIMISM_CHAIN_ID,
  },
  {
    address: "0xF9680D99D6C9589e2a9f15691456aD908bF5d836",
    symbol: "ETH/USD",
    chainId: POLYGON_CHAIN_ID,
  },
  {
    address: "0xc907E116054Ad103354f2D350FD2514433D57F6f",
    symbol: "BTC/USD",
    chainId: POLYGON_CHAIN_ID,
  },
  {
    address: "0x71041dddad3595F9CEd3DcCFBe3D1F4b0a16Bb70",
    symbol: "ETH/USD",
    chainId: BASE_CHAIN_ID,
  },
  {
    address: "0x07B6ea7aB3C0FbcB29be25381a8C0f885e347895",
    symbol: "BTC/USD",
    chainId: BASE_CHAIN_ID,
  },
  {
    address: "0x639Fe6ab55C921f74e7fac1ee960C0B6293ba612",
    symbol: "ETH/USD",
    chainId: ARBITRUM_CHAIN_ID,
  },
  {
    address: "0x6ce185860a496ce102b7325b39ce47432d6dbdfd",
    symbol: "BTC/USD",
    chainId: ARBITRUM_CHAIN_ID,
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
