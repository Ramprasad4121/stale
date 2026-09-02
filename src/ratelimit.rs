use crate::types::GuardrailResult;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    max_tx: usize,
    window: Duration,
    timestamps: Vec<Instant>,
}

impl RateLimiter {
    pub fn new(max_tx: usize, window_seconds: u64) -> Result<Self, String> {
        if max_tx == 0 {
            return Err("max_tx must be greater than 0".to_string());
        }
        if window_seconds == 0 {
            return Err("window_seconds must be greater than 0".to_string());
        }

        Ok(Self {
            max_tx,
            window: Duration::from_secs(window_seconds),
            timestamps: Vec::new(),
        })
    }

    pub fn check(&mut self) -> GuardrailResult {
        let now = Instant::now();
        let window = self.window;
        self.timestamps.retain(|&t| now.duration_since(t) < window);

        if self.timestamps.len() >= self.max_tx {
            GuardrailResult::block(format!(
                "RATE LIMIT EXCEEDED: {}/{} transactions in the last {}s window. — BLOCK",
                self.timestamps.len(),
                self.max_tx,
                self.window.as_secs()
            ))
        } else {
            GuardrailResult::allow(format!(
                "rate limit ok ({}/{})",
                self.timestamps.len(),
                self.max_tx
            ))
        }
    }

    pub fn record(&mut self) {
        self.timestamps.push(Instant::now());
    }

    pub fn remaining(&mut self) -> usize {
        let now = Instant::now();
        let window = self.window;
        self.timestamps.retain(|&t| now.duration_since(t) < window);
        self.max_tx.saturating_sub(self.timestamps.len())
    }
}

pub struct SpendingCap {
    max_spend: u128,
    window: Duration,
    ledger: Vec<(Instant, u128)>,
}

impl SpendingCap {
    pub fn new(max_spend: u128, window_seconds: u64) -> Result<Self, String> {
        if max_spend == 0 {
            return Err("max_spend must be greater than 0".to_string());
        }
        if window_seconds == 0 {
            return Err("window_seconds must be greater than 0".to_string());
        }

        Ok(Self {
            max_spend,
            window: Duration::from_secs(window_seconds),
            ledger: Vec::new(),
        })
    }

    pub fn check(&mut self, proposed_amount: u128) -> GuardrailResult {
        let now = Instant::now();
        let window = self.window;
        self.ledger.retain(|&(t, _)| now.duration_since(t) < window);

        let current_spend: u128 = self
            .ledger
            .iter()
            .fold(0u128, |acc, &(_, amt)| acc.saturating_add(amt));

        let projected_spend = match current_spend.checked_add(proposed_amount) {
            Some(sum) => sum,
            None => return GuardrailResult::block("overflow in projected spend — BLOCK"),
        };

        if projected_spend > self.max_spend {
            GuardrailResult::block(format!(
                "SPENDING CAP EXCEEDED: projected spend {} exceeds cap {} in the last {}s window. — BLOCK",
                projected_spend, self.max_spend, self.window.as_secs()
            ))
        } else {
            GuardrailResult::allow(format!(
                "spending cap ok ({}/{})",
                current_spend, self.max_spend
            ))
        }
    }

    pub fn record(&mut self, amount: u128) {
        self.ledger.push((Instant::now(), amount));
    }

    pub fn remaining(&mut self) -> u128 {
        let now = Instant::now();
        let window = self.window;
        self.ledger.retain(|&(t, _)| now.duration_since(t) < window);
        let spent: u128 = self
            .ledger
            .iter()
            .fold(0u128, |acc, &(_, amt)| acc.saturating_add(amt));
        self.max_spend.saturating_sub(spent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(2, 60).unwrap();
        assert!(limiter.check().allow_execute);
        limiter.record();
        assert!(limiter.check().allow_execute);
        limiter.record();
        assert!(!limiter.check().allow_execute);
        assert_eq!(limiter.remaining(), 0);
    }

    #[test]
    fn test_spending_cap() {
        let mut cap = SpendingCap::new(1000, 60).unwrap();
        assert!(cap.check(400).allow_execute);
        cap.record(400);
        assert_eq!(cap.remaining(), 600);
        assert!(!cap.check(700).allow_execute);
        assert!(cap.check(600).allow_execute);
    }
}
