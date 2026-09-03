//! `stale-mcp`: stdio JSON-RPC/MCP server exposing guardrails to agents.
//!
//! # BLOCK contract (read this before integrating)
//! A `BLOCK` verdict is returned as `result.content[0].text` (pretty JSON
//! containing `"decision": "Block"`) **with `isError: true`**. Agents MUST
//! treat `isError == true` as "do not execute" — checking only the absence
//! of a protocol `error` field is a fail-open integration bug.
//!
//! # Input bounds
//! Each stdin line is capped at [`MAX_MCP_LINE_BYTES`] (1 MiB); oversized
//! lines yield a `-32600` error instead of buffering unbounded memory.

use serde_json::{json, Value};
use stale::check::{check_price, CheckPriceInput};
use stale::is_stale::{is_stale, IsStaleInput};
use stale::quote::{quote_from_feed, QuoteInput};
use stale::rpc::HttpRpcClient;
use stale::types::Decision;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Max bytes per JSON-RPC request line (DoS bound).
pub const MAX_MCP_LINE_BYTES: usize = 1024 * 1024;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin).lines();
    let mut stdout = tokio::io::stdout();

    while let Ok(Some(line)) = reader.next_line().await {
        if line.len() > MAX_MCP_LINE_BYTES {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32600, "message": format!("request exceeds {} byte limit", MAX_MCP_LINE_BYTES) }
            });
            let _ = stdout.write_all(format!("{}\n", err_resp).as_bytes()).await;
            let _ = stdout.flush().await;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(e) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": format!("Parse error: {}", e) }
                });
                let _ = stdout.write_all(format!("{}\n", err_resp).as_bytes()).await;
                let _ = stdout.flush().await;
                continue;
            }
        };

        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let response = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "stale",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                }
            }),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": [
                        {
                            "name": "stale_isStale",
                            "description": "Check if a Chainlink price is stale. Pure, no RPC. Returns ALLOW/BLOCK with age and reason.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "updatedAt": { "type": "string", "description": "updatedAt timestamp as string or number" },
                                    "nowSeconds": { "type": "integer", "description": "Current time in seconds" },
                                    "maxAgeSeconds": { "type": "integer", "description": "Max allowed age in seconds" }
                                },
                                "required": ["updatedAt", "nowSeconds", "maxAgeSeconds"]
                            }
                        },
                        {
                            "name": "stale_quote",
                            "description": "Price math from Data Feed answer + decimals. No RPC. Returns priceUsd and quoteUsd.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "answer": { "type": "string", "description": "answer as string" },
                                    "decimals": { "type": "integer", "description": "Feed decimals" },
                                    "amountEth": { "type": "number", "description": "Human ETH amount for quote" }
                                },
                                "required": ["answer", "decimals"]
                            }
                        },
                        {
                            "name": "stale_check",
                            "description": "Full guardrail: Chainlink latestRoundData + decimals -> isStale -> quote. Fail closed.",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "rpc": { "type": "string", "description": "Ethereum RPC URL" },
                                    "feed": { "type": "string", "description": "Data Feed proxy address" },
                                    "maxAgeSeconds": { "type": "integer", "description": "Max allowed age in seconds" },
                                    "amountEth": { "type": "number", "description": "Human ETH amount for quote" },
                                    "nowSeconds": { "type": "integer", "description": "Override now timestamp" }
                                },
                                "required": ["rpc", "feed", "maxAgeSeconds"]
                            }
                        }
                    ]
                }
            }),
            "tools/call" => {
                let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(Value::Null);

                match tool_name {
                    "stale_isStale" => {
                        let updated_at_str = args.get("updatedAt").and_then(|v| {
                            if let Some(s) = v.as_str() {
                                Some(s.to_string())
                            } else {
                                v.as_i64().map(|n| n.to_string())
                            }
                        });
                        let now_seconds = match args.get("nowSeconds").and_then(|v| v.as_i64()) {
                            Some(n) => n,
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": "BLOCK — missing nowSeconds"}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };
                        let max_age_seconds = match args
                            .get("maxAgeSeconds")
                            .and_then(|v| v.as_i64())
                        {
                            Some(n) => n,
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": "BLOCK — missing maxAgeSeconds"}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };

                        let parsed_updated_at = updated_at_str.and_then(|s| {
                            if s.starts_with("0x") || s.starts_with("0X") {
                                i64::from_str_radix(
                                    s.trim_start_matches("0x").trim_start_matches("0X"),
                                    16,
                                )
                                .ok()
                            } else {
                                s.parse::<i64>().ok()
                            }
                        });

                        let res = is_stale(IsStaleInput {
                            updated_at: parsed_updated_at,
                            now_seconds,
                            max_age_seconds,
                        });
                        let blocked = res.decision == Decision::Block;

                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&res).unwrap()
                                }],
                                "isError": blocked
                            }
                        })
                    }
                    "stale_quote" => {
                        let answer_str = args.get("answer").and_then(|v| v.as_str()).unwrap_or("");
                        let decimals_raw = match args.get("decimals").and_then(|v| v.as_u64()) {
                            Some(d) => d,
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": "BLOCK — quote failed: missing decimals (query the feed's decimals() on-chain, never assume)"}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };
                        let amount = args.get("amountEth").and_then(|v| v.as_f64());

                        if decimals_raw > 36 {
                            let _ = stdout.write_all(format!("{}\n", json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{"type": "text", "text": "BLOCK — quote failed: invalid decimals (max 36; query the feed's decimals() on-chain, never assume)"}],
                                    "isError": true
                                }
                            })).as_bytes()).await;
                            let _ = stdout.flush().await;
                            continue;
                        }
                        let decimals = decimals_raw as u8;
                        let answer = match parse_answer(answer_str) {
                            Ok(a) => a,
                            Err(_) => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": "BLOCK — quote failed: unparseable answer"}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };
                        match quote_from_feed(QuoteInput {
                            answer,
                            decimals,
                            amount,
                        }) {
                            Ok(res) => json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": serde_json::to_string_pretty(&res).unwrap()
                                    }]
                                }
                            }),
                            Err(e) => json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": format!("BLOCK — quote failed: {}", e)
                                    }],
                                    "isError": true
                                }
                            }),
                        }
                    }
                    "stale_check" => {
                        let rpc = args.get("rpc").and_then(|v| v.as_str()).unwrap_or("");
                        let feed = args.get("feed").and_then(|v| v.as_str()).unwrap_or("");
                        let max_age_seconds = match args
                            .get("maxAgeSeconds")
                            .and_then(|v| v.as_i64())
                        {
                            Some(n) => n,
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": "BLOCK — missing maxAgeSeconds (no silent default; caller policy required)"}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };
                        let amount_eth = args.get("amountEth").and_then(|v| v.as_f64());
                        let now_seconds = args.get("nowSeconds").and_then(|v| v.as_i64());

                        let client = HttpRpcClient::new(rpc);
                        let res = check_price(
                            &client,
                            CheckPriceInput {
                                feed,
                                max_age_seconds,
                                amount_eth,
                                now_seconds,
                            },
                        )
                        .await;
                        let blocked = res.decision == Decision::Block;

                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&res).unwrap()
                                }],
                                "isError": blocked
                            }
                        })
                    }
                    _ => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": format!("Tool not found: {}", tool_name) }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {}", method) }
            }),
        };

        let _ = stdout.write_all(format!("{}\n", response).as_bytes()).await;
        let _ = stdout.flush().await;
    }
}

/// Parse a decimal or `0x`/`0X` hex answer string into `i128`.
fn parse_answer(s: &str) -> Result<i128, String> {
    let t = s.trim();
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        // Signed hex: interpret via i128 two's complement if high bit set.
        let u = u128::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
        Ok(u as i128)
    } else {
        t.parse::<i128>().map_err(|e| e.to_string())
    }
}
