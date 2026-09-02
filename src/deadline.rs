use crate::types::GuardrailResult;

#[derive(Debug, Clone)]
pub struct CheckDeadlineInput {
    pub deadline: i64,
    pub max_future_seconds: Option<i64>,
    pub min_future_seconds: Option<i64>,
}

pub fn check_deadline(input: CheckDeadlineInput, now_seconds: i64) -> GuardrailResult {
    let max_future_seconds = input.max_future_seconds.unwrap_or(1200);
    let min_future_seconds = input.min_future_seconds.unwrap_or(30);

    if input.deadline <= 0 {
        return GuardrailResult::block("invalid deadline — BLOCK");
    }
    if max_future_seconds <= 0 {
        return GuardrailResult::block("invalid maxFutureSeconds — BLOCK");
    }
    if min_future_seconds < 0 {
        return GuardrailResult::block("invalid minFutureSeconds — BLOCK");
    }

    let delta = input.deadline - now_seconds;

    if delta < 0 {
        return GuardrailResult::block(format!(
            "EXPIRED DEADLINE: deadline is {} seconds in the past. The agent is replaying a stale intent. — BLOCK",
            delta.abs()
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
}
