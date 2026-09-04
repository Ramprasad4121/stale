use clap::{Parser, Subcommand};
use serde::Serialize;
use stale::check::{check_price, CheckPriceInput};
use stale::feeds::{lookup_feed, DEFAULT_FEED, REGISTRY};
use stale::is_stale::{is_stale, IsStaleInput};
use stale::quote::{quote_from_feed, QuoteInput};
use stale::rpc::HttpRpcClient;

#[derive(Parser, Debug)]
#[command(
    name = "stale",
    version,
    about = "DeFi fail-closed guardrails for autonomous AI agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Ethereum RPC URL
    #[arg(long)]
    rpc: Option<String>,

    /// Max allowed age in seconds
    #[arg(long)]
    max_age: Option<i64>,

    /// Comma-separated RPC hosts allowed for egress (SSRF guard).
    /// Unset = unrestricted (legacy).
    #[arg(long)]
    allowed_rpc_hosts: Option<String>,

    /// Data Feed proxy address
    #[arg(long, default_value = DEFAULT_FEED)]
    feed: String,

    /// Human amount for quote (e.g. 0.5)
    #[arg(long)]
    amount: Option<f64>,

    /// Output single JSON object
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check a Chainlink price feed
    Check {
        /// Ethereum RPC URL
        #[arg(long)]
        rpc: String,
        /// Max allowed age in seconds
        #[arg(long)]
        max_age: i64,
        /// Data Feed proxy address
        #[arg(long, default_value = DEFAULT_FEED)]
        feed: String,
        /// Human amount for quote (e.g. 0.5)
        #[arg(long)]
        amount: Option<f64>,
        /// Output single JSON object
        #[arg(long)]
        json: bool,
        /// Comma-separated RPC hosts allowed for egress (SSRF guard).
        /// Unset = unrestricted (legacy).
        #[arg(long)]
        allowed_rpc_hosts: Option<String>,
    },
    /// Test staleness calculation without RPC
    IsStale {
        /// Feed updatedAt timestamp (unix seconds)
        #[arg(long)]
        updated_at: i64,
        /// Max allowed age in seconds
        #[arg(long)]
        max_age: i64,
        /// Override now timestamp (default: system clock)
        #[arg(long)]
        now: Option<i64>,
    },
    /// Compute price quote from raw feed answer and decimals
    Quote {
        /// Raw feed answer: decimal (e.g. 250000000000) or 0x hex (≤ i128::MAX)
        #[arg(long, value_parser = parse_answer)]
        answer: i128,
        /// Feed decimals (0-36)
        #[arg(long)]
        decimals: u8,
        /// Human amount for quote (e.g. 0.5)
        #[arg(long)]
        amount: Option<f64>,
    },
    /// List all recognized feeds
    Feeds,
}

/// Serialize for CLI output. Serialization of our own result types is
/// infallible in practice, but binaries must never panic (constitution:
/// zero runtime panics), so failures degrade to a JSON error stub.
fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| "{\"error\":\"response serialization failed\"}".to_string())
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // Top-level flags are only read when NO subcommand is given. Warn
    // instead of silently dropping a second flag set.
    if cli.command.is_some() && (cli.rpc.is_some() || cli.max_age.is_some()) {
        eprintln!(
            "Warning: top-level --rpc/--max-age are ignored when a subcommand is given; \
             the subcommand's own flags apply."
        );
    }

    match cli.command {
        Some(Commands::Check {
            rpc,
            max_age,
            feed,
            amount,
            json,
            allowed_rpc_hosts,
        }) => {
            if max_age < 0 {
                eprintln!("Error: --max-age must be >= 0 (got {max_age})");
                std::process::exit(2);
            }
            run_check(&rpc, max_age, &feed, amount, json, allowed_rpc_hosts).await;
        }
        Some(Commands::IsStale {
            updated_at,
            max_age,
            now,
        }) => {
            if max_age < 0 {
                eprintln!("Error: --max-age must be >= 0 (got {max_age})");
                std::process::exit(2);
            }
            let now_sec = now.unwrap_or_else(|| chrono::Utc::now().timestamp());
            let res = is_stale(IsStaleInput {
                updated_at: Some(updated_at),
                now_seconds: now_sec,
                max_age_seconds: max_age,
            });
            println!("{}", to_json(&res));
            if res.decision == stale::types::Decision::Block {
                std::process::exit(1);
            }
        }
        Some(Commands::Quote {
            answer,
            decimals,
            amount,
        }) => match quote_from_feed(QuoteInput {
            answer,
            decimals,
            amount,
        }) {
            Ok(res) => println!("{}", to_json(&res)),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        },
        Some(Commands::Feeds) => {
            println!("{}", to_json(&REGISTRY));
        }
        None => {
            if let (Some(rpc), Some(max_age)) = (cli.rpc, cli.max_age) {
                run_check(
                    &rpc,
                    max_age,
                    &cli.feed,
                    cli.amount,
                    cli.json,
                    cli.allowed_rpc_hosts,
                )
                .await;
            } else {
                eprintln!("Error: missing required arguments --rpc and --max-age");
                eprintln!("Run `stale --help` for usage information.");
                // Usage error (exit 2), distinct from guard verdicts:
                // 0 = ALLOW, 1 = BLOCK, 2 = usage/config error.
                std::process::exit(2);
            }
        }
    }
}

/// Parse `--answer`: decimal or `0x` hex (≤ `i128::MAX`). Anything else
/// (including out-of-range hex) is a usage error (exit 2), never a quote.
fn parse_answer(s: &str) -> Result<i128, String> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        if hex.is_empty() || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(format!("invalid hex answer {:?}", s));
        }
        let magnitude = u128::from_str_radix(hex, 16)
            .map_err(|e| format!("invalid hex answer {:?}: {}", s, e))?;
        return i128::try_from(magnitude)
            .map_err(|_| format!("hex answer {:?} exceeds i128::MAX", s));
    }
    t.parse::<i128>()
        .map_err(|e| format!("invalid decimal answer {:?}: {}", s, e))
}

/// Parse `--allowed-rpc-hosts a,b` into a host list. Exported for reuse.
pub fn parse_allowed_hosts(raw: Option<String>) -> Vec<String> {
    raw.unwrap_or_default()
        .split(',')
        .map(|h| h.trim().to_string())
        .filter(|h| !h.is_empty())
        .collect()
}

async fn run_check(
    rpc: &str,
    max_age: i64,
    feed: &str,
    amount: Option<f64>,
    json: bool,
    allowed_rpc_hosts: Option<String>,
) {
    let client = HttpRpcClient::new(rpc).with_allowed_hosts(parse_allowed_hosts(allowed_rpc_hosts));
    let res = check_price(
        &client,
        CheckPriceInput {
            feed,
            max_age_seconds: max_age,
            amount_eth: amount,
            now_seconds: None,
        },
    )
    .await;

    if json {
        println!("{}", to_json(&res));
    } else {
        println!("decision:     {}", res.decision);
        println!("reason:       {}", res.reason);
        println!("allowExecute: {}", res.allow_execute);
        if let Some(price) = res.price_usd {
            println!("priceUsd:     ${:.2}", price);
        }
        if let Some(quote) = res.quote_usd {
            println!("quoteUsd:     ${:.2}", quote);
        }
        if let Some(symbol) = lookup_feed(feed).map(|f| f.symbol) {
            println!("feedSymbol:   {}", symbol);
        }
    }

    if res.decision == stale::types::Decision::Block {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_answer_decimal_and_hex() {
        assert_eq!(parse_answer("250000000000").unwrap(), 250000000000);
        assert_eq!(parse_answer("  42 ").unwrap(), 42);
        assert_eq!(parse_answer("-7").unwrap(), -7);
        assert_eq!(parse_answer("0xff").unwrap(), 255);
        assert_eq!(parse_answer("0XFF").unwrap(), 255);
        assert!(parse_answer("0x").is_err());
        assert!(parse_answer("0xZZ").is_err());
        assert!(parse_answer("abc").is_err());
        // Beyond i128::MAX must be a usage error, never a wrap.
        assert!(parse_answer("0xffffffffffffffffffffffffffffffff").is_err());
    }

    #[test]
    fn test_parse_allowed_hosts_splits_and_trims() {
        assert!(parse_allowed_hosts(None).is_empty());
        assert_eq!(
            parse_allowed_hosts(Some("a.example, b.example ,, ".to_string())),
            vec!["a.example".to_string(), "b.example".to_string()]
        );
    }
}
