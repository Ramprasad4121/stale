//! Integer slippage engine: `minAmountOut` from oracle prices + bps.
//!
//! Pure integer math (no `f64`): `raw = amountIn * priceIn / priceOut`
//! rescaled across token/oracle decimals, then `min = raw * (1 - slip)`.
//! Every step is checked — any overflow fails closed with `Err` (→ BLOCK).
//!
//! A zero result is rejected: `minAmountOut == 0` downstream authorizes
//! total-loss fills.

use serde::{Deserialize, Serialize};

/// Max supported decimal exponent diff. `10^38` already exceeds `u128::MAX`
/// (`≈3.4e38`), so anything above 38 cannot scale without overflow — the
/// `checked_pow` below enforces this; the constant documents intent.
pub const MAX_EXP_DIFF: u32 = 38;

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

/// Compute `minAmountOut` (all amounts in base units).
///
/// # Errors
/// `Err` on zero prices, `slippage_bps > 10000`, `slippage_bps == 10000`
/// (100% allows total loss), `amount_in == 0`, zero/exponent-overflow
/// results. Callers map `Err` → `BLOCK`.
pub fn calculate_min_amount_out(input: CalculateMinAmountOutInput) -> Result<u128, String> {
    if input.amount_in == 0 {
        return Err("amount_in must be > 0 (zero input yields zero output) — BLOCK".to_string());
    }
    if input.price_in_answer == 0 {
        return Err("price_in_answer must be > 0".to_string());
    }
    if input.price_out_answer == 0 {
        return Err("price_out_answer must be > 0".to_string());
    }
    if input.slippage_bps > 10000 {
        return Err("slippage_bps cannot exceed 10000 (100%)".to_string());
    }
    if input.slippage_bps == 10000 {
        return Err("slippage_bps of 10000 (100%) would allow total loss — BLOCK".to_string());
    }

    let exp_out: i64 = (input.token_out_decimals as i64) + (input.price_out_decimals as i64);
    let exp_in: i64 = (input.token_in_decimals as i64) + (input.price_in_decimals as i64);
    let exp_diff = exp_out
        .checked_sub(exp_in)
        .ok_or_else(|| "overflow in decimal exponent diff".to_string())?;

    let prod = input
        .amount_in
        .checked_mul(input.price_in_answer)
        .ok_or_else(|| "overflow in amount_in * price_in_answer".to_string())?;

    let raw_amount_out = if exp_diff >= 0 {
        let exp_u32: u32 = u32::try_from(exp_diff)
            .ok()
            .filter(|&e| e <= MAX_EXP_DIFF)
            .ok_or_else(|| "decimal exponent out of range".to_string())?;
        let scale = 10u128
            .checked_pow(exp_u32)
            .ok_or_else(|| "overflow in decimal scale".to_string())?;
        // Compute (prod * scale) / price_out without premature truncation:
        // Using: (prod * scale) / price_out = (prod / price_out) * scale + ((prod % price_out) * scale) / price_out
        let q = prod / input.price_out_answer;
        let r = prod % input.price_out_answer;

        let term1 = q
            .checked_mul(scale)
            .ok_or_else(|| "overflow in scaling quotient".to_string())?;
        let term2 = r
            .checked_mul(scale)
            .ok_or_else(|| "overflow in scaling remainder".to_string())?
            / input.price_out_answer;

        term1
            .checked_add(term2)
            .ok_or_else(|| "overflow in raw_amount_out".to_string())?
    } else {
        // exp_diff < 0 here (checked above), so negation cannot overflow.
        let neg = exp_diff.checked_neg().ok_or_else(|| {
            "overflow in decimal exponent negation (unreachable) — BLOCK".to_string()
        })?;
        let exp_u32: u32 = u32::try_from(neg)
            .ok()
            .filter(|&e| e <= MAX_EXP_DIFF)
            .ok_or_else(|| "decimal exponent out of range".to_string())?;
        let scale = 10u128
            .checked_pow(exp_u32)
            .ok_or_else(|| "overflow in decimal scale".to_string())?;
        match input.price_out_answer.checked_mul(scale) {
            Some(denom) => prod / denom,
            None => (prod / input.price_out_answer) / scale,
        }
    };

    let factor = (10000 - input.slippage_bps) as u128;
    let min_amount_out = raw_amount_out
        .checked_mul(factor)
        .ok_or_else(|| "overflow in slippage factor".to_string())?
        / 10000;

    if min_amount_out == 0 {
        return Err(
            "computed minAmountOut is 0 (dust input or extreme decimals) — BLOCK".to_string(),
        );
    }

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
    fn test_small_amount_preserves_precision() {
        // amount_in * price_in < price_out
        // 10 wei of tokenIn priced at 2000, swapping to tokenOut priced at 50000, exp_diff = +6
        let res = calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 10,
            token_in_decimals: 6,
            price_in_answer: 2000,
            price_in_decimals: 0,
            token_out_decimals: 12,
            price_out_answer: 50000,
            price_out_decimals: 0,
            slippage_bps: 0,
        })
        .unwrap();

        // (10 * 2000 * 10^6) / 50000 = 20_000_000_000 / 50000 = 400_000
        assert_eq!(res, 400_000);
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
        // 100% slippage allows total loss — must also fail
        assert!(calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 100,
            token_in_decimals: 18,
            price_in_answer: 100,
            price_in_decimals: 8,
            token_out_decimals: 18,
            price_out_answer: 100,
            price_out_decimals: 8,
            slippage_bps: 10000,
        })
        .is_err());
    }

    #[test]
    fn test_zero_amount_in_rejected() {
        assert!(calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 0,
            token_in_decimals: 18,
            price_in_answer: 100,
            price_in_decimals: 8,
            token_out_decimals: 18,
            price_out_answer: 100,
            price_out_decimals: 8,
            slippage_bps: 50,
        })
        .is_err());
    }

    #[test]
    fn test_dust_yielding_zero_output_rejected() {
        // 1 wei through a 1:1e18 price ratio truncates to 0 → must Err, not Ok(0).
        assert!(calculate_min_amount_out(CalculateMinAmountOutInput {
            amount_in: 1,
            token_in_decimals: 18,
            price_in_answer: 1,
            price_in_decimals: 8,
            token_out_decimals: 18,
            price_out_answer: 1_000_000_000_000_000_000,
            price_out_decimals: 8,
            slippage_bps: 0,
        })
        .is_err());
    }
}
