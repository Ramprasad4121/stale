use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculateMinAmountOutInput {
    pub amount_in: u128,
    pub token_in_decimals: u32,
    pub price_in_answer: u128,
    pub price_in_decimals: u32,

    pub token_out_decimals: u32,
    pub price_out_answer: u128,
    pub price_out_decimals: u32,

    /// Slippage tolerance in basis points. E.g. 50 = 0.5%
    pub slippage_bps: u32,
}

pub fn calculate_min_amount_out(input: CalculateMinAmountOutInput) -> Result<u128, String> {
    if input.price_in_answer == 0 {
        return Err("price_in_answer must be > 0".to_string());
    }
    if input.price_out_answer == 0 {
        return Err("price_out_answer must be > 0".to_string());
    }
    if input.slippage_bps > 10000 {
        return Err("slippage_bps cannot exceed 10000 (100%)".to_string());
    }

    let exp_out = (input.token_out_decimals + input.price_out_decimals) as i32;
    let exp_in = (input.token_in_decimals + input.price_in_decimals) as i32;
    let exp_diff = exp_out - exp_in;

    let base_out = input
        .amount_in
        .checked_mul(input.price_in_answer)
        .ok_or_else(|| "overflow in amount_in * price_in_answer".to_string())?
        / input.price_out_answer;

    let raw_amount_out = if exp_diff >= 0 {
        let scale = 10u128.pow(exp_diff as u32);
        base_out
            .checked_mul(scale)
            .ok_or_else(|| "overflow in scaling raw_amount_out".to_string())?
    } else {
        let scale = 10u128.pow((-exp_diff) as u32);
        base_out / scale
    };

    let factor = (10000 - input.slippage_bps) as u128;
    let min_amount_out = (raw_amount_out * factor) / 10000;

    Ok(min_amount_out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zero_slippage() {
        // 1 WETH (18 decimals) priced at $2500 (8 decimals)
        // Swap to USDC (6 decimals) priced at $1 (8 decimals)
        let res = calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 1_000_000_000_000_000_000, // 1 WETH
            token_in_decimals: 18,
            price_in_answer: 2500_00000000,
            price_in_decimals: 8,
            token_out_decimals: 6,
            price_out_answer: 1_00000000,
            price_out_decimals: 8,
            slippage_bps: 0,
        })
        .unwrap();

        // 2500 USDC with 6 decimals = 2_500_000_000
        assert_eq!(res, 2_500_000_000);
    }

    #[test]
    fn test_with_50_bps_slippage() {
        let res = calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 1_000_000_000_000_000_000,
            token_in_decimals: 18,
            price_in_answer: 2500_00000000,
            price_in_decimals: 8,
            token_out_decimals: 6,
            price_out_answer: 1_00000000,
            price_out_decimals: 8,
            slippage_bps: 50, // 0.5%
        })
        .unwrap();

        // 2500 * (1 - 0.005) = 2487.5 USDC = 2_487_500_000
        assert_eq!(res, 2_487_500_000);
    }

    #[test]
    fn test_invalid_slippage_fails() {
        assert!(calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 100,
            token_in_decimals: 18,
            price_in_answer: 100,
            price_in_decimals: 8,
            token_out_decimals: 18,
            price_out_answer: 100,
            price_out_decimals: 8,
            slippage_bps: 10001,
        })
        .is_err());
    }
}
