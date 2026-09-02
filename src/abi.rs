pub fn encode_address_param(addr: &str) -> String {
    let clean = addr.trim().trim_start_matches("0x");
    format!("{:0>64}", clean)
}

pub fn encode_u256_param(val: u128) -> String {
    format!("{:0>64x}", val)
}

pub fn decode_word_u128(hex_str: &str, offset_words: usize) -> Result<u128, String> {
    let clean = hex_str.trim().trim_start_matches("0x");
    let start = offset_words * 64;
    let end = start + 64;
    if clean.len() < end {
        return Err(format!("hex response too short for word {}", offset_words));
    }
    let slice = &clean[start..end];
    u128::from_str_radix(slice, 16).map_err(|e| format!("failed to parse word u128: {}", e))
}

pub fn decode_word_i128(hex_str: &str, offset_words: usize) -> Result<i128, String> {
    let clean = hex_str.trim().trim_start_matches("0x");
    let start = offset_words * 64;
    let end = start + 64;
    if clean.len() < end {
        return Err(format!("hex response too short for word {}", offset_words));
    }
    let slice = &clean[start..end];
    // Check two's complement if first character >= '8'
    let first_char = slice.chars().next().unwrap_or('0');
    if ('8'..='f').contains(&first_char) || ('A'..='F').contains(&first_char) {
        // Negative number in 256-bit representation
        let u = u128::from_str_radix(&slice[32..64], 16)
            .map_err(|e| format!("failed to parse negative i128: {}", e))?;
        // If it's a signed 256 bit negative number, it's negative
        let signed = -(1i128.wrapping_add(!u as i128));
        Ok(signed)
    } else {
        u128::from_str_radix(&slice[32..64], 16)
            .map(|u| u as i128)
            .map_err(|e| format!("failed to parse word i128: {}", e))
    }
}

pub fn decode_bool(hex_str: &str) -> Result<bool, String> {
    let word = decode_word_u128(hex_str, 0)?;
    Ok(word != 0)
}

pub fn decode_round_data(hex_str: &str) -> Result<(u128, i128, u128, u128, u128), String> {
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
        let encoded = encode_address_param("0x5f4eC3Df9cbd43714FE2740f5E3616155c5b8419");
        assert_eq!(encoded.len(), 64);
        assert!(encoded.ends_with("5f4eC3Df9cbd43714FE2740f5E3616155c5b8419"));
    }

    #[test]
    fn test_decode_round_data() {
        // 5 words of 64 hex chars = 320 hex chars
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
}
