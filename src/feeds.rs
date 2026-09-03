//! Static Chainlink registry: feed proxies + L2 sequencer uptime feeds.
//!
//! Addresses are matched case-insensitively by [`lookup_feed`]. Adding a
//! feed is a registry change: address, symbol, chain id, tests, and the
//! SECURITY doc update together.

use serde::{Deserialize, Serialize};

pub const MAINNET_CHAIN_ID: u64 = 1;
pub const OPTIMISM_CHAIN_ID: u64 = 10;
pub const POLYGON_CHAIN_ID: u64 = 137;
pub const BASE_CHAIN_ID: u64 = 8453;
pub const ARBITRUM_CHAIN_ID: u64 = 42161;
pub const ZKSYNC_CHAIN_ID: u64 = 324;
pub const METIS_CHAIN_ID: u64 = 1088;
pub const MANTLE_CHAIN_ID: u64 = 5000;
pub const SCROLL_CHAIN_ID: u64 = 534352;

pub const DEFAULT_FEED: &str = "0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419";
pub const BTC_USD_FEED: &str = "0xF4030086522a5bEEa4988F8cA5B36dbC97BeE88c";
pub const USDC_USD_FEED: &str = "0x8fFfFfd4AfB6115b954Bd326cbe7B4BA576818f6";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
/// One allowlisted feed proxy: checksummed address, pair symbol, chain id.
pub struct FeedEntry {
    pub address: &'static str,
    pub symbol: &'static str,
    pub chain_id: u64,
}

pub const REGISTRY: &[FeedEntry] = &[
    FeedEntry {
        address: DEFAULT_FEED,
        symbol: "ETH/USD",
        chain_id: MAINNET_CHAIN_ID,
    },
    FeedEntry {
        address: BTC_USD_FEED,
        symbol: "BTC/USD",
        chain_id: MAINNET_CHAIN_ID,
    },
    FeedEntry {
        address: USDC_USD_FEED,
        symbol: "USDC/USD",
        chain_id: MAINNET_CHAIN_ID,
    },
    FeedEntry {
        address: "0x13e3Ee699D1909E989722E753853AE30b17e08c5",
        symbol: "ETH/USD",
        chain_id: OPTIMISM_CHAIN_ID,
    },
    FeedEntry {
        address: "0x718A5788b89454aAE3A028AE9c111A29Be6c2aB0",
        symbol: "BTC/USD",
        chain_id: OPTIMISM_CHAIN_ID,
    },
    FeedEntry {
        address: "0xF9680D99D6C9589e2a9f15691456aD908bF5d836",
        symbol: "ETH/USD",
        chain_id: POLYGON_CHAIN_ID,
    },
    FeedEntry {
        address: "0xc907E116054Ad103354f2D350FD2514433D57F6f",
        symbol: "BTC/USD",
        chain_id: POLYGON_CHAIN_ID,
    },
    FeedEntry {
        address: "0x71041dddad3595F9CEd3DcCFBe3D1F4b0a16Bb70",
        symbol: "ETH/USD",
        chain_id: BASE_CHAIN_ID,
    },
    FeedEntry {
        address: "0x07B6ea7aB3C0FbcB29be25381a8C0f885e347895",
        symbol: "BTC/USD",
        chain_id: BASE_CHAIN_ID,
    },
    FeedEntry {
        address: "0x639Fe6ab55C921f74e7fac1ee960C0B6293ba612",
        symbol: "ETH/USD",
        chain_id: ARBITRUM_CHAIN_ID,
    },
    FeedEntry {
        address: "0x6ce185860a496ce102b7325b39ce47432d6dbdfd",
        symbol: "BTC/USD",
        chain_id: ARBITRUM_CHAIN_ID,
    },
    FeedEntry {
        address: "0x6D41d1dc81ea29853289D222B5D5B3f2F4f75727",
        symbol: "ETH/USD",
        chain_id: ZKSYNC_CHAIN_ID,
    },
    FeedEntry {
        address: "0xBd04B7fA53B3C18cbA3C088b9076C374b344C811",
        symbol: "ETH/USD",
        chain_id: METIS_CHAIN_ID,
    },
    FeedEntry {
        address: "0x32357754B4B99aB123fE7778b4dcA2E351E2cb2B",
        symbol: "ETH/USD",
        chain_id: MANTLE_CHAIN_ID,
    },
    FeedEntry {
        address: "0x59F1ec1f10bD7eD9B938431086bC1D9e233ECf41",
        symbol: "ETH/USD",
        chain_id: SCROLL_CHAIN_ID,
    },
];

/// Sequencer uptime feed for an L2 chain, or `None` (mainnet / unknown).
pub fn get_sequencer_feed(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        ARBITRUM_CHAIN_ID => Some("0xFdB631F5EE196F0ed6FAa767959853A9F217697D"),
        OPTIMISM_CHAIN_ID => Some("0x371EAD81c9102C9BF4874A9075FFFf170F2Ee389"),
        BASE_CHAIN_ID => Some("0xBCF85224fc0756B9Fa45aA7892530B47e10b6433"),
        ZKSYNC_CHAIN_ID => Some("0x0E6AC8B967393dcD3D36677c126976157F993940"),
        METIS_CHAIN_ID => Some("0x58218ea7422255EBE94e56b504035a784b7AA204"),
        MANTLE_CHAIN_ID => Some("0xaDE1b9AbB98c6A542E4B49db2588a3Ec4bF7Cdf0"),
        SCROLL_CHAIN_ID => Some("0x45c2b8C204568A03Dc7A2E32B71D67Fe97F908A9"),
        _ => None,
    }
}

/// Case-insensitive registry lookup. `None` → caller emits BLOCK
/// ("unknown feed / not allowlisted").
pub fn lookup_feed(address: &str) -> Option<&'static FeedEntry> {
    let lower = address.to_lowercase();
    REGISTRY.iter().find(|e| e.address.to_lowercase() == lower)
}

pub const FEEDS: &[FeedEntry] = REGISTRY;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_feed_found() {
        let entry = lookup_feed(DEFAULT_FEED).unwrap();
        assert_eq!(entry.symbol, "ETH/USD");
        assert_eq!(entry.chain_id, 1);
    }

    #[test]
    fn test_lookup_feed_case_insensitive() {
        let entry = lookup_feed(&DEFAULT_FEED.to_lowercase()).unwrap();
        assert_eq!(entry.symbol, "ETH/USD");
    }

    #[test]
    fn test_lookup_feed_not_found() {
        assert!(lookup_feed("0x0000000000000000000000000000000000000000").is_none());
    }

    #[test]
    fn test_sequencer_feeds() {
        assert!(get_sequencer_feed(ARBITRUM_CHAIN_ID).is_some());
        assert_eq!(get_sequencer_feed(MAINNET_CHAIN_ID), None);
    }
}
