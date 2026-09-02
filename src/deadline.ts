/**
 * @module deadline
 * Swap Deadline Guardrail.
 *
 * Ensures that transaction deadlines set by the agent are reasonable:
 * - Not already expired (agent is replaying stale intents)
 * - Not too far in the future (exposes the agent to long-lived MEV risk)
 *
 * This is pure, synchronous, zero-dependency.
 */

export type DeadlineGuardrailResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export type CheckDeadlineInput = {
  /** The deadline timestamp in seconds (Unix epoch) */
  deadline: number;
  /** Maximum allowed seconds into the future. Default: 1200 (20 minutes) */
  maxFutureSeconds?: number;
  /** Minimum allowed seconds into the future. Default: 30 */
  minFutureSeconds?: number;
};

/**
 * Validates that a swap deadline is reasonable.
 * Fails closed (BLOCK) if the deadline is expired or excessively far in the future.
 */
export function checkDeadline(input: CheckDeadlineInput): DeadlineGuardrailResult {
  const { deadline, maxFutureSeconds = 1200, minFutureSeconds = 30 } = input;

  if (typeof deadline !== "number" || !Number.isFinite(deadline) || deadline <= 0) {
    return { decision: "BLOCK", reason: "invalid deadline — BLOCK", allowExecute: false };
  }
  if (typeof maxFutureSeconds !== "number" || maxFutureSeconds <= 0) {
    return { decision: "BLOCK", reason: "invalid maxFutureSeconds — BLOCK", allowExecute: false };
  }
  if (typeof minFutureSeconds !== "number" || minFutureSeconds < 0) {
    return { decision: "BLOCK", reason: "invalid minFutureSeconds — BLOCK", allowExecute: false };
  }

  const nowSeconds = Math.floor(Date.now() / 1000);
  const delta = deadline - nowSeconds;

  if (delta < 0) {
    return {
      decision: "BLOCK",
      reason: `EXPIRED DEADLINE: deadline is ${Math.abs(delta)} seconds in the past. The agent is replaying a stale intent. — BLOCK`,
      allowExecute: false,
    };
  }

  if (delta < minFutureSeconds) {
    return {
      decision: "BLOCK",
      reason: `DEADLINE TOO TIGHT: deadline is only ${delta}s away (min ${minFutureSeconds}s). Transaction will likely expire before confirmation. — BLOCK`,
      allowExecute: false,
    };
  }

  if (delta > maxFutureSeconds) {
    return {
      decision: "BLOCK",
      reason: `DEADLINE TOO FAR: deadline is ${delta}s into the future (max ${maxFutureSeconds}s). Long-lived deadlines expose the agent to MEV risk. — BLOCK`,
      allowExecute: false,
    };
  }

  return {
    decision: "ALLOW",
    reason: `deadline is ${delta}s in the future (acceptable range)`,
    allowExecute: true,
  };
}
