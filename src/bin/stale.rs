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
        /// Raw feed answer (signed integer)
        #[arg(long)]
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

    match cli.command {
        Some(Commands::Check {
            rpc,
            max_age,
            feed,
            amount,
            json,
            allowed_rpc_hosts,
        }) => {
            run_check(&rpc, max_age, &feed, amount, json, allowed_rpc_hosts).await;
        }
        Some(Commands::IsStale {
            updated_at,
            max_age,
            now,
        }) => {
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
            Ok(res) => println!("{}", serde_json::to_string_pretty(&res).unwrap()),
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
