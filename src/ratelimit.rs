//! In-memory rolling-window resource governors.
//!
//! [`RateLimiter`] caps transaction *frequency*; [`SpendingCap`] caps
//! cumulative *value*. Both are fail-closed advisory guards — see caveats.
//!
//! # Correctness caveats (read before relying on these)
//! - **Not thread-safe across tasks:** methods take `&mut self`. Share via
//!   `Mutex`/`RwLock` in concurrent agents.
//! - **`check` + `record` is check-then-act:** two concurrent `check()` calls
//!   can both pass before either `record()`s. Prefer the atomic
//!   [`RateLimiter::try_acquire`] / [`SpendingCap::try_spend`].
//! - **Volatile:** state lives in RAM and resets on restart. A restart
//!   clears the window — persist externally if restart-bypass matters.
//! - **Bounded:** histories are capped at [`MAX_HISTORY`] entries; excess
//!   oldest entries are evicted first (fail-closed direction for limiter).

use crate::types::GuardrailResult;
use std::time::{Duration, Instant};

/// Hard bound on retained timestamps/ledger entries (DoS cap).
pub const MAX_HISTORY: usize = 100_000;

/// Rolling-window transaction frequency governor.
///
/// ```rust
/// use stale::ratelimit::RateLimiter;
/// let mut limiter = RateLimiter::new(10, 60).unwrap();
/// assert!(limiter.try_acquire().allow_execute);
/// ```
pub struct RateLimiter {
    max_tx: usize,
    window: Duration,
    timestamps: Vec<Instant>,
}

impl RateLimiter {
    /// Create a limiter allowing `max_tx` transactions per `window_seconds`.
    ///
    /// # Errors
    /// `Err` if `max_tx == 0` or `window_seconds == 0`.
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

    /// Non-mutating admission test. Prefer [`try_acquire`](Self::try_acquire)
    /// in production: `check` followed by a separate `record` is TOCTOU —
    /// a caller that forgets `record()` is never limited.
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

    /// Record a transaction (manual mode). Callers using [`check`](Self::check)
    /// must call this on every allowed transaction; otherwise the limiter
    /// never fills. Prefer [`try_acquire`](Self::try_acquire).
    pub fn record(&mut self) {
        self.timestamps.push(Instant::now());
        if self.timestamps.len() > MAX_HISTORY {
            let excess = self.timestamps.len() - MAX_HISTORY;
            self.timestamps.drain(..excess);
        }
    }

    /// Atomic check-and-record. Returns `Allow` (slot consumed) or `Block`
    /// (no slot consumed). This is the recommended entry point.
    pub fn try_acquire(&mut self) -> GuardrailResult {
        let verdict = self.check();
        if verdict.allow_execute {
            self.record();
        }
        verdict
    }

    /// Slots remaining in the current window.
    pub fn remaining(&mut self) -> usize {
        let now = Instant::now();
        let window = self.window;
        self.timestamps.retain(|&t| now.duration_since(t) < window);
        self.max_tx.saturating_sub(self.timestamps.len())
    }
}

/// Rolling-window cumulative spending governor (saturating `u128` math).
pub struct SpendingCap {
    max_spend: u128,
    window: Duration,
    ledger: Vec<(Instant, u128)>,
}

impl SpendingCap {
    /// Create a cap allowing `max_spend` base units per `window_seconds`.
    ///
    /// # Errors
    /// `Err` if `max_spend == 0` or `window_seconds == 0`.
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

    /// Non-mutating admission test for `proposed_amount`. Same TOCTOU
    /// caveat as [`RateLimiter::check`]: prefer [`try_spend`](Self::try_spend).
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

    /// Record spend (manual mode). See [`try_spend`](Self::try_spend).
    pub fn record(&mut self, amount: u128) {
        self.ledger.push((Instant::now(), amount));
        if self.ledger.len() > MAX_HISTORY {
            let excess = self.ledger.len() - MAX_HISTORY;
            self.ledger.drain(..excess);
        }
    }

    /// Atomic check-and-record. Returns `Allow` (amount booked) or `Block`
    /// (nothing booked). Recommended entry point.
    pub fn try_spend(&mut self, amount: u128) -> GuardrailResult {
        let verdict = self.check(amount);
        if verdict.allow_execute {
            self.record(amount);
        }
        verdict
    }

    /// Spendable remainder in the current window (saturating).
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

    #[test]
    fn test_try_acquire_atomic() {
        let mut limiter = RateLimiter::new(1, 60).unwrap();
        assert!(limiter.try_acquire().allow_execute);
        // Slot consumed atomically — second acquire BLOCKs without manual record.
        assert!(!limiter.try_acquire().allow_execute);
    }

    #[test]
    fn test_try_spend_atomic() {
        let mut cap = SpendingCap::new(1000, 60).unwrap();
        assert!(cap.try_spend(600).allow_execute);
        assert!(!cap.try_spend(500).allow_execute);
        assert!(cap.try_spend(400).allow_execute);
    }
}
