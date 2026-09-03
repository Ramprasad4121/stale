//! Transaction deadline guard.
//!
//! Rejects expired intents (replay), too-tight windows (expiry before
//! confirmation), and far-future windows (long-lived MEV exposure).

use crate::types::GuardrailResult;

#[derive(Debug, Clone)]
/// Deadline policy. Defaults: `min_future 30s`, `max_future 1200s`.
pub struct CheckDeadlineInput {
    pub deadline: i64,
    pub max_future_seconds: Option<i64>,
    pub min_future_seconds: Option<i64>,
}

/// Validate `deadline` against `now_seconds`.
///
/// `delta = deadline - now` (checked math) must lie in
/// `[min_future_seconds, max_future_seconds]`. Negative `now_seconds`
/// and `i64::MIN` edge cases fail closed without panicking.
pub fn check_deadline(input: CheckDeadlineInput, now_seconds: i64) -> GuardrailResult {
    let max_future_seconds = input.max_future_seconds.unwrap_or(1200);
    let min_future_seconds = input.min_future_seconds.unwrap_or(30);

    if now_seconds < 0 {
        return GuardrailResult::block("invalid now timestamp (negative) — BLOCK");
    }

    if input.deadline <= 0 {
        return GuardrailResult::block("invalid deadline — BLOCK");
    }
    if max_future_seconds <= 0 {
        return GuardrailResult::block("invalid maxFutureSeconds — BLOCK");
    }
    if min_future_seconds < 0 {
        return GuardrailResult::block("invalid minFutureSeconds — BLOCK");
    }

    if min_future_seconds > max_future_seconds {
        return GuardrailResult::block(
            "invalid deadline window: minFutureSeconds > maxFutureSeconds — BLOCK",
        );
    }

    let delta = match input.deadline.checked_sub(now_seconds) {
        Some(d) => d,
        None => {
            return GuardrailResult::block(
                "deadline timestamp arithmetic overflow — BLOCK (fail closed)",
            )
        }
    };

    if delta < 0 {
        return GuardrailResult::block(format!(
            "EXPIRED DEADLINE: deadline is {} seconds in the past. The agent is replaying a stale intent. — BLOCK",
            delta.unsigned_abs()
        ));
    }

    if delta < min_future_seconds {
        return GuardrailResult::block(format!(
            "DEADLINE TOO TIGHT: deadline is only {}s away (min {}s). Transaction will likely expire before confirmation. — BLOCK",
            delta, min_future_seconds
        ));
    }

    if delta > max_future_seconds {
        return GuardrailResult::block(format!(
            "DEADLINE TOO FAR: deadline is {}s into the future (max {}s). Long-lived deadlines expose the agent to MEV risk. — BLOCK",
            delta, max_future_seconds
        ));
    }

    GuardrailResult::allow(format!(
        "deadline is {}s in the future (acceptable range)",
        delta
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasonable_deadline() {
        let now = 1000;
        let res = check_deadline(
            CheckDeadlineInput {
                deadline: now + 300,
                max_future_seconds: None,
                min_future_seconds: None,
            },
            now,
        );
        assert!(res.allow_execute);
    }

    #[test]
    fn test_expired_deadline() {
        let now = 1000;
        let res = check_deadline(
            CheckDeadlineInput {
                deadline: now - 10,
                max_future_seconds: None,
                min_future_seconds: None,
            },
            now,
        );
        assert!(!res.allow_execute);
        assert!(res.reason.contains("EXPIRED DEADLINE"));
    }

    #[test]
    fn test_deadline_too_tight() {
        let now = 1000;
        let res = check_deadline(
            CheckDeadlineInput {
                deadline: now + 15,
                max_future_seconds: None,
                min_future_seconds: Some(30),
            },
            now,
        );
        assert!(!res.allow_execute);
        assert!(res.reason.contains("DEADLINE TOO TIGHT"));
    }

    #[test]
    fn test_negative_now_blocks_fail_closed() {
        let res = check_deadline(
            CheckDeadlineInput {
                deadline: 1000,
                max_future_seconds: None,
                min_future_seconds: None,
            },
            -5,
        );
        assert!(!res.allow_execute);
    }

    #[test]
    fn test_i64_min_delta_no_panic() {
        // deadline=i64::MIN, now=i64::MAX would overflow a naive sub; checked
        // math must BLOCK instead of panicking (release overflow-checks on).
        let res = check_deadline(
            CheckDeadlineInput {
                deadline: i64::MIN,
                max_future_seconds: None,
                min_future_seconds: None,
            },
            i64::MAX,
        );
        assert!(!res.allow_execute);
    }
}
