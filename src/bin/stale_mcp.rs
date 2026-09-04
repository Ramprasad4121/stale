//! `stale-mcp`: stdio JSON-RPC/MCP server exposing guardrails to agents.
//!
//! # BLOCK contract (read this before integrating)
//! A `BLOCK` verdict is returned as `result.content[0].text` (pretty JSON
//! containing `"decision": "BLOCK"`) **with `isError: true`**. Agents MUST
//! treat `isError == true` as "do not execute" — checking only the absence
//! of a protocol `error` field is a fail-open integration bug.
//!
//! Every `content[0].text` is JSON-parseable: guard verdicts serialize the
//! result struct; bad-argument rejections serialize
//! `{"decision":"BLOCK","reason":"…"}`. Never assume plain text.
//!
//! # Input bounds
//! Each stdin line is capped at [`MAX_MCP_LINE_BYTES`] (1 MiB); oversized
//! lines yield a `-32600` error instead of buffering unbounded memory.

/// Serialize for MCP text content. Never panic on serialization failure;
/// degrade to a JSON error stub (fail closed at the transport layer).
fn to_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string_pretty(value)
        .unwrap_or_else(|_| "{\"error\":\"response serialization failed\"}".to_string())
}

/// Bad-argument BLOCK payload, JSON shaped like a guard verdict so every
/// `content[0].text` is uniformly `JSON.parse`-able (never plain text).
fn arg_block_text(reason: &str) -> String {
    json!({"decision": "BLOCK", "reason": reason}).to_string()
}

use serde::Serialize;
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
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line_buf: Vec<u8> = Vec::new();

    loop {
        line_buf.clear();
        // Bounded line read: never buffer more than cap+1 bytes for one
        // request. `BufReader::lines()` would heap the whole line BEFORE
        // any length check — a newline-less flood is unbounded memory.
        let mut oversized = false;
        let mut eof = false;
        loop {
            let available = match reader.fill_buf().await {
                Ok(buf) => buf,
                Err(_) => {
                    // Stdin transport error: say so on stdout, then exit.
                    // (A dead pipe with no frame is undebuggable.)
                    let _ = stdout
                        .write_all(
                            format!(
                                "{}\n",
                                json!({
                                    "jsonrpc": "2.0",
                                    "id": null,
                                    "error": { "code": -32603, "message": "stdin transport error" }
                                })
                            )
                            .as_bytes(),
                        )
                        .await;
                    let _ = stdout.flush().await;
                    return;
                }
            };
            if available.is_empty() {
                eof = true;
                break;
            }
            match available.iter().position(|&b| b == b'\n') {
                Some(nl) => {
                    if line_buf.len() + nl + 1 > MAX_MCP_LINE_BYTES {
                        oversized = true;
                    } else {
                        line_buf.extend_from_slice(&available[..=nl]);
                    }
                    reader.consume(nl + 1);
                    break;
                }
                None => {
                    if line_buf.len() + available.len() > MAX_MCP_LINE_BYTES {
                        oversized = true;
                    } else {
                        line_buf.extend_from_slice(available);
                    }
                    let n = available.len();
                    reader.consume(n);
                }
            }
            if oversized {
                // Discard the rest of the overlong line in bounded
                // chunks, then emit -32600 below.
                loop {
                    let rest = match reader.fill_buf().await {
                        Ok(buf) => buf,
                        Err(_) => return,
                    };
                    if rest.is_empty() {
                        break;
                    }
                    match rest.iter().position(|&b| b == b'\n') {
                        Some(nl) => {
                            reader.consume(nl + 1);
                            break;
                        }
                        None => {
                            let n = rest.len();
                            reader.consume(n);
                        }
                    }
                }
                break;
            }
        }
        if eof && line_buf.is_empty() {
            return;
        }
        if oversized {
            let err_resp = json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32600, "message": format!("request exceeds {} byte limit", MAX_MCP_LINE_BYTES) }
            });
            let _ = stdout.write_all(format!("{}\n", err_resp).as_bytes()).await;
            let _ = stdout.flush().await;
            continue;
        }
        let line = match String::from_utf8(std::mem::take(&mut line_buf)) {
            Ok(s) => s,
            Err(_) => {
                let err_resp = json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": { "code": -32700, "message": "request is not valid UTF-8" }
                });
                let _ = stdout.write_all(format!("{}\n", err_resp).as_bytes()).await;
                let _ = stdout.flush().await;
                continue;
            }
        };
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
                                    "updatedAt": { "type": ["string", "integer"], "description": "updatedAt timestamp as decimal/0x string or integer" },
                                    "nowSeconds": { "type": ["integer", "string"], "description": "Current time in seconds (integer or numeric string)" },
                                    "maxAgeSeconds": { "type": ["integer", "string"], "description": "Max allowed age in seconds (integer or numeric string)" }
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
                                    "answer": { "type": ["string", "integer"], "description": "answer as decimal/0x string or integer (floats rejected)" },
                                    "decimals": { "type": ["integer", "string"], "description": "Feed decimals as integer or numeric string" },
                                    "amountEth": { "type": ["number", "string"], "description": "Human ETH amount for quote (number or numeric string)" }
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
                                    "maxAgeSeconds": { "type": ["integer", "string"], "description": "Max allowed age in seconds (integer or numeric string)" },
                                    "amountEth": { "type": ["number", "string"], "description": "Human ETH amount for quote (number or numeric string)" },
                                    "nowSeconds": { "type": ["integer", "string"], "description": "Override now timestamp (integer or numeric string)" },
                                    "allowedRpcHosts": { "type": "array", "items": { "type": "string" }, "description": "Egress allowlist for the RPC URL host (SSRF guard). Unset = unrestricted." }
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
                        let updated_at_str = match args.get("updatedAt") {
                            // Absent stays absent: is_stale BLOCKs on
                            // missing input (no silent default).
                            None | Some(Value::Null) => None,
                            Some(v) => match coerce_answer_str(v) {
                                Some(s) => Some(s),
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid updatedAt (expected decimal/0x string or integer)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let now_seconds = match args.get("nowSeconds") {
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — missing nowSeconds")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                            Some(v) => match coerce_i64(v) {
                                Some(n) => n,
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid nowSeconds (expected integer seconds)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let max_age_seconds = match args.get("maxAgeSeconds") {
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — missing maxAgeSeconds")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                            Some(v) => match coerce_i64(v) {
                                Some(n) => n,
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid maxAgeSeconds (expected integer seconds)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
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
                                    "text": to_json(&res)
                                }],
                                "isError": blocked
                            }
                        })
                    }
                    "stale_quote" => {
                        let answer_str = match args.get("answer") {
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: missing answer (decimal/0x string or integer)")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                            Some(v) => match coerce_answer_str(v) {
                                Some(s) => s,
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: invalid answer (expected decimal/0x string or integer; floats rejected)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let decimals_raw = match args.get("decimals") {
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: missing decimals (query the feed's decimals() on-chain, never assume)")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                            Some(v) => match coerce_u64(v) {
                                Some(d) => d,
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: invalid decimals (expected integer 0-36; query the feed's decimals() on-chain, never assume)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let amount = match args.get("amountEth") {
                            // Absent means "no quote requested", not zero.
                            None | Some(Value::Null) => None,
                            Some(v) => match coerce_f64(v) {
                                Some(f) => Some(f),
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: invalid amountEth (expected number or numeric string)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };

                        if decimals_raw > 36 {
                            let _ = stdout.write_all(format!("{}\n", json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: invalid decimals (max 36; query the feed's decimals() on-chain, never assume)")}],
                                    "isError": true
                                }
                            })).as_bytes()).await;
                            let _ = stdout.flush().await;
                            continue;
                        }
                        let decimals = decimals_raw as u8;
                        let answer = match parse_answer(&answer_str) {
                            Ok(a) => a,
                            Err(_) => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — quote failed: unparseable answer")}],
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
                                        "text": to_json(&res)
                                    }],
                                    // Uniform verdict contract: success
                                    // carries explicit isError false.
                                    "isError": false
                                }
                            }),
                            Err(e) => json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": arg_block_text(&format!("BLOCK — quote failed: {}", e))
                                    }],
                                    "isError": true
                                }
                            }),
                        }
                    }
                    "stale_check" => {
                        let rpc = match args.get("rpc") {
                            Some(Value::String(s)) if !s.trim().is_empty() => s.clone(),
                            _ => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — missing rpc (Ethereum RPC URL is required)")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };
                        let feed = match args.get("feed") {
                            Some(Value::String(s)) if !s.trim().is_empty() => s.clone(),
                            _ => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — missing feed (Data Feed proxy address is required)")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };
                        let max_age_seconds = match args.get("maxAgeSeconds") {
                            None => {
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — missing maxAgeSeconds (no silent default; caller policy required)")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                            Some(v) => match coerce_i64(v) {
                                Some(n) => n,
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid maxAgeSeconds (expected integer seconds)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let amount_eth = match args.get("amountEth") {
                            None | Some(Value::Null) => None,
                            Some(v) => match coerce_f64(v) {
                                Some(f) => Some(f),
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid amountEth (expected number or numeric string)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let now_seconds = match args.get("nowSeconds") {
                            None | Some(Value::Null) => None,
                            Some(v) => match coerce_i64(v) {
                                Some(n) => Some(n),
                                None => {
                                    let _ = stdout.write_all(format!("{}\n", json!({
                                        "jsonrpc": "2.0",
                                        "id": id,
                                        "result": {
                                            "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid nowSeconds (expected integer seconds)")}],
                                            "isError": true
                                        }
                                    })).as_bytes()).await;
                                    let _ = stdout.flush().await;
                                    continue;
                                }
                            },
                        };
                        let allowed_hosts: Vec<String> = match args.get("allowedRpcHosts") {
                            None | Some(Value::Null) => Vec::new(),
                            Some(Value::Array(arr)) => arr
                                .iter()
                                .filter_map(|h| h.as_str())
                                .map(|h| h.trim().to_string())
                                .filter(|h| !h.is_empty())
                                .collect(),
                            Some(_) => {
                                // Wrong type must BLOCK, never silently
                                // downgrade to unrestricted egress.
                                let _ = stdout.write_all(format!("{}\n", json!({
                                    "jsonrpc": "2.0",
                                    "id": id,
                                    "result": {
                                        "content": [{"type": "text", "text": arg_block_text("BLOCK — invalid allowedRpcHosts (expected array of host strings)")}],
                                        "isError": true
                                    }
                                })).as_bytes()).await;
                                let _ = stdout.flush().await;
                                continue;
                            }
                        };

                        let client = HttpRpcClient::new(&rpc).with_allowed_hosts(allowed_hosts);
                        let res = check_price(
                            &client,
                            CheckPriceInput {
                                feed: feed.as_str(),
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
                                    "text": to_json(&res)
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

/// Coerce a JSON arg to `i64`: JSON integer, u64 in range, integral f64,
/// or decimal/`0x` string. Anything else (float, bool, null, object) is
/// `None` so the caller reports invalid instead of silently defaulting.
fn coerce_i64(v: &Value) -> Option<i64> {
    if let Some(n) = v.as_i64() {
        return Some(n);
    }
    if let Some(n) = v.as_u64() {
        return i64::try_from(n).ok();
    }
    if let Some(f) = v.as_f64() {
        if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
            return Some(f as i64);
        }
        return None;
    }
    if let Some(s) = v.as_str() {
        return parse_answer(s).ok()?.try_into().ok();
    }
    None
}

/// Coerce a JSON arg to `u64`: JSON integer, integral f64, or
/// decimal/`0x` string.
fn coerce_u64(v: &Value) -> Option<u64> {
    if let Some(n) = v.as_u64() {
        return Some(n);
    }
    if let Some(n) = v.as_i64() {
        return u64::try_from(n).ok();
    }
    if let Some(f) = v.as_f64() {
        if f.fract() == 0.0 && f >= 0.0 && f <= u64::MAX as f64 {
            return Some(f as u64);
        }
        return None;
    }
    if let Some(s) = v.as_str() {
        let a = parse_answer(s).ok()?;
        return u64::try_from(a).ok();
    }
    None
}

/// Coerce a JSON arg to `f64`: JSON number or numeric string.
/// Non-finite results are left to `quote_from_feed`, which BLOCKs them.
fn coerce_f64(v: &Value) -> Option<f64> {
    if let Some(f) = v.as_f64() {
        return Some(f);
    }
    if let Some(s) = v.as_str() {
        return s.trim().parse::<f64>().ok();
    }
    None
}

/// Coerce an answer arg: decimal/`0x` string or JSON integer.
/// Floats are rejected (precision loss) — the caller reports invalid.
fn coerce_answer_str(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        return Some(s.to_string());
    }
    if let Some(n) = v.as_i64() {
        return Some(n.to_string());
    }
    if let Some(n) = v.as_u64() {
        return Some(n.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_coerce_i64_accepts_int_and_numeric_string() {
        assert_eq!(coerce_i64(&json!(60)), Some(60));
        assert_eq!(coerce_i64(&json!("60")), Some(60));
        assert_eq!(coerce_i64(&json!("0x3c")), Some(60));
        assert_eq!(coerce_i64(&json!(60.0)), Some(60));
        assert_eq!(coerce_i64(&json!(u64::MAX)), None);
        assert_eq!(coerce_i64(&json!(60.5)), None);
        assert_eq!(coerce_i64(&json!(true)), None);
        assert_eq!(coerce_i64(&json!(null)), None);
    }

    #[test]
    fn test_coerce_u64_accepts_string_decimals() {
        assert_eq!(coerce_u64(&json!(8)), Some(8));
        assert_eq!(coerce_u64(&json!("8")), Some(8));
        assert_eq!(coerce_u64(&json!(-1)), None);
        assert_eq!(coerce_u64(&json!(8.5)), None);
    }

    #[test]
    fn test_coerce_f64_accepts_string_amount() {
        assert_eq!(coerce_f64(&json!(0.5)), Some(0.5));
        assert_eq!(coerce_f64(&json!("0.5")), Some(0.5));
        assert_eq!(coerce_f64(&json!("abc")), None);
        assert_eq!(coerce_f64(&json!(true)), None);
    }

    #[test]
    fn test_coerce_answer_str_accepts_integers_rejects_floats() {
        assert_eq!(
            coerce_answer_str(&json!(245377000000i64)),
            Some("245377000000".to_string())
        );
        assert_eq!(
            coerce_answer_str(&json!("245377000000")),
            Some("245377000000".to_string())
        );
        assert_eq!(coerce_answer_str(&json!(1.5)), None);
        assert_eq!(coerce_answer_str(&json!(true)), None);
    }

    #[test]
    fn test_arg_block_text_is_json_shaped_verdict() {
        // Every content[0].text must be JSON.parse-able by integrators.
        let text = arg_block_text("BLOCK — missing rpc");
        let parsed: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed["decision"], "BLOCK");
        assert!(parsed["reason"].as_str().unwrap().contains("missing rpc"));
    }

    #[test]
    fn test_nan_amount_blocked_downstream() {
        // coerce_f64 accepts "NaN"; the quote guard must still BLOCK it.
        let nan = coerce_f64(&json!("NaN")).unwrap();
        assert!(nan.is_nan());
        let res = quote_from_feed(QuoteInput {
            answer: 2500_00000000,
            decimals: 8,
            amount: Some(nan),
        });
        assert!(res.is_err());
    }
}
