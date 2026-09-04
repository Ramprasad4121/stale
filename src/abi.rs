//! Minimal ABI codec for the selectors `stale` needs.
//!
//! # Bounds
//! - `uint256` values are decoded to `u128`; on-chain values above
//!   `u128::MAX` (e.g. `type(uint256).max` approvals) FAIL with an overflow
//!   error instead of truncating. Callers needing full 256-bit range must
//!   use a U256 path (see `check_approval` docs).
//! - `int256` values are decoded to `i128` with strict two's-complement
//!   validation; out-of-range values fail closed.
//! - Single-word decoders accept trailing words (multivalue returns read by
//!   offset); [`decode_round_data`] requires exactly 5 words.

/// ABI-encode an `address` param as one 32-byte word (lowercased hex).
///
/// # Errors
/// `Err` unless the input is `0x` + exactly 40 hex chars (overlong input
/// is rejected, never truncated).
pub fn encode_address_param(addr: &str) -> Result<String, String> {
    let clean = addr
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if clean.len() != 40 {
        return Err(format!(
            "invalid address length {} (expected 40 hex chars) — BLOCK",
            clean.len()
        ));
    }
    if !clean.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("invalid address: non-hex characters — BLOCK".to_string());
    }
    Ok(format!("{:0>64}", clean.to_lowercase()))
}

/// ABI-encode a `uint256` param from a `u128`.
///
/// Values above `u128::MAX` cannot be represented — this is a deliberate
/// bound (see module docs), not silent truncation.
pub fn encode_u256_param(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// Decode the `offset_words`-th 32-byte word as `u128`.
///
/// Accepts `0x`/`0X`/bare hex; `Err` on short input or values `> u128::MAX`.
pub fn decode_word_u128(hex_str: &str, offset_words: usize) -> Result<u128, String> {
    let clean = hex_str
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if !clean.is_ascii() {
        return Err("hex response contains non-ASCII characters".to_string());
    }
    // Checked offset math: a huge `offset_words` must yield Err, never a
    // debug-build arithmetic panic under `overflow-checks = true`.
    let start = offset_words
        .checked_mul(64)
        .ok_or_else(|| format!("word offset {} overflows — BLOCK", offset_words))?;
    let end = start
        .checked_add(64)
        .ok_or_else(|| format!("word offset {} overflows — BLOCK", offset_words))?;
    let slice = clean
        .get(start..end)
        .ok_or_else(|| format!("hex response too short for word {}", offset_words))?;

    u128::from_str_radix(slice, 16).map_err(|e| format!("failed to parse word u128: {}", e))
}

/// Decode the `offset_words`-th word as `i128` (two's complement).
///
/// Positive values require a zero upper half; negatives require an `0xff…`
/// upper half with the sign bit set below. Out-of-`i128`-range `Err`s.
pub fn decode_word_i128(hex_str: &str, offset_words: usize) -> Result<i128, String> {
    let clean = hex_str
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if !clean.is_ascii() {
        return Err("hex response contains non-ASCII characters".to_string());
    }
    let start = offset_words
        .checked_mul(64)
        .ok_or_else(|| format!("word offset {} overflows — BLOCK", offset_words))?;
    let end = start
        .checked_add(64)
        .ok_or_else(|| format!("word offset {} overflows — BLOCK", offset_words))?;
    let slice = clean
        .get(start..end)
        .ok_or_else(|| format!("hex response too short for word {}", offset_words))?;

    let upper = &slice[0..32];
    let lower = &slice[32..64];

    // Check two's complement if first character >= '8'
    let first_char = slice.chars().next().unwrap_or('0');
    if ('8'..='f').contains(&first_char) || ('A'..='F').contains(&first_char) {
        // Negative number in 256-bit representation
        // Upper 128 bits MUST all be 'f' or 'F' to fit in i128
        if !upper.chars().all(|c| c == 'f' || c == 'F') {
            return Err("int256 underflow: negative value exceeds i128 bounds".to_string());
        }
        let u = u128::from_str_radix(lower, 16)
            .map_err(|e| format!("failed to parse negative i128: {}", e))?;
        // Must have MSB set in lower half as well
        if u < (1u128 << 127) {
            return Err("int256 underflow: corrupted sign bit in lower 128 bits".to_string());
        }
        let signed = u as i128;
        Ok(signed)
    } else {
        // Positive number in 256-bit representation
        // Upper 128 bits MUST all be '0'
        if !upper.chars().all(|c| c == '0') {
            return Err("int256 overflow: positive value exceeds i128 bounds".to_string());
        }
        let u = u128::from_str_radix(lower, 16)
            .map_err(|e| format!("failed to parse word i128: {}", e))?;
        if u > (i128::MAX as u128) {
            return Err("int256 overflow: positive value exceeds i128::MAX".to_string());
        }
        Ok(u as i128)
    }
}

/// Decode a `bool` word: `0` → false, any nonzero → true (fail-closed
/// direction for `paused`/`sanctioned` consumers).
pub fn decode_bool(hex_str: &str) -> Result<bool, String> {
    let word = decode_word_u128(hex_str, 0)?;
    Ok(word != 0)
}

/// Decode Chainlink `latestRoundData()` →
/// `(roundId, answer, startedAt, updatedAt, answeredInRound)`.
///
/// Requires exactly 5 words; `Err` otherwise (multivalue trailing-word
/// tolerance does NOT apply here — round shape is consensus-critical).
pub fn decode_round_data(hex_str: &str) -> Result<(u128, i128, u128, u128, u128), String> {
    let clean = hex_str
        .trim()
        .trim_start_matches("0x")
        .trim_start_matches("0X");
    if clean.len() != 5 * 64 {
        return Err(format!(
            "invalid latestRoundData length {} (expected {}) — BLOCK",
            clean.len(),
            5 * 64
        ));
    }
    let round_id = decode_word_u128(hex_str, 0)?;
    let answer = decode_word_i128(hex_str, 1)?;
    let started_at = decode_word_u128(hex_str, 2)?;
    let updated_at = decode_word_u128(hex_str, 3)?;
    let answered_in_round = decode_word_u128(hex_str, 4)?;
    Ok((round_id, answer, started_at, updated_at, answered_in_round))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_address() {
        let encoded = encode_address_param("0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419").unwrap();
        assert_eq!(encoded.len(), 64);
        assert!(encoded.ends_with("5f4ec3df9cbd43714fe2740f5e3616155c5b8419"));
    }

    #[test]
    fn test_encode_address_rejects_invalid() {
        assert!(encode_address_param("0x1234").is_err());
        assert!(encode_address_param("0xZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ").is_err());
        // Must not silently truncate overlong input
        assert!(encode_address_param("0x00005f4eC3Df9cbd43714FE2740f5E3616155c5b8419").is_err());
    }

    #[test]
    fn test_decode_round_data() {
        let round_id = format!("{:0>64x}", 1u64);
        let answer = format!("{:0>64x}", 250000000000u64);
        let started_at = format!("{:0>64x}", 1700000000u64);
        let updated_at = format!("{:0>64x}", 1700000050u64);
        let answered_in_round = format!("{:0>64x}", 1u64);

        let hex = format!(
            "0x{}{}{}{}{}",
            round_id, answer, started_at, updated_at, answered_in_round
        );
        let decoded = decode_round_data(&hex).unwrap();
        assert_eq!(decoded.0, 1);
        assert_eq!(decoded.1, 250000000000);
        assert_eq!(decoded.2, 1700000000);
        assert_eq!(decoded.3, 1700000050);
        assert_eq!(decoded.4, 1);
    }

    #[test]
    fn test_decode_negative_int256() {
        // -1 in 256 bits = 64 'f' characters
        let minus_one_hex = "f".repeat(64);
        let decoded = decode_word_i128(&minus_one_hex, 0).unwrap();
        assert_eq!(decoded, -1);
    }

    #[test]
    fn test_decode_int256_overflow_rejected() {
        // Positive value 2^128 (upper bit set in word)
        let mut overflow_hex = "0".repeat(31) + "1" + &"0".repeat(32);
        assert!(decode_word_i128(&overflow_hex, 0).is_err());

        // Negative value < -2^127
        overflow_hex = "8".repeat(32) + &"0".repeat(32);
        assert!(decode_word_i128(&overflow_hex, 0).is_err());
    }

    #[test]
    fn test_decode_huge_offset_errors_instead_of_panicking() {
        let word = format!("0x{:0>64x}", 1u64);
        // offset*64 overflows usize: must be Err, never a panic under
        // overflow-checks (fail closed at decode time).
        assert!(decode_word_u128(&word, usize::MAX).is_err());
        assert!(decode_word_i128(&word, usize::MAX).is_err());
    }
}
