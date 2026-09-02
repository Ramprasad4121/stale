use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuoteResult {
    pub price_usd: f64,
    pub quote_usd: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct QuoteInput {
    pub answer: i128,
    pub decimals: u8,
    pub amount: Option<f64>,
}

pub fn quote_from_feed(input: QuoteInput) -> Result<QuoteResult, String> {
    if input.answer <= 0 {
        return Err(format!("invalid non-positive answer {}", input.answer));
    }
    if input.decimals > 36 {
        return Err(format!("invalid decimals {}", input.decimals));
    }
    if let Some(amt) = input.amount {
        if !amt.is_finite() || amt < 0.0 {
            return Err(format!("invalid amount {}", amt));
        }
    }

    let divisor = 10_f64.powi(input.decimals as i32);
    let price_usd = (input.answer as f64) / divisor;

    if !price_usd.is_finite() || price_usd < 0.0 {
        return Err(format!("invalid calculated price_usd {}", price_usd));
    }

    let quote_usd = match input.amount {
        Some(amt) => {
            let q = amt * price_usd;
            if !q.is_finite() {
                return Err("calculated quote_usd is not finite".to_string());
            }
            Some(q)
        }
        None => None,
    };

    Ok(QuoteResult {
        price_usd,
        quote_usd,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_quote() {
        let res = quote_from_feed(QuoteInput {
            answer: 245377000000,
            decimals: 8,
            amount: Some(0.5),
        })
        .unwrap();

        assert!((res.price_usd - 2453.77).abs() < 1e-4);
        assert!((res.quote_usd.unwrap() - 1226.885).abs() < 1e-3);
    }

    #[test]
    fn test_zero_or_negative_answer_fails() {
        assert!(quote_from_feed(QuoteInput {
            answer: 0,
            decimals: 8,
            amount: None,
        })
        .is_err());

        assert!(quote_from_feed(QuoteInput {
            answer: -100,
            decimals: 8,
            amount: None,
        })
        .is_err());
    }

    #[test]
    fn test_excessive_decimals_fails() {
        assert!(quote_from_feed(QuoteInput {
            answer: 1000,
            decimals: 37,
            amount: None,
        })
        .is_err());
    }
}
