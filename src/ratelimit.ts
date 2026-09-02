/**
 * @module ratelimit
 * Rate Limiter & Spending Cap Guardrails.
 *
 * Prevents runaway AI agents from executing an unlimited number of transactions
 * or spending more than a configured maximum within a rolling time window.
 *
 * These are pure in-memory guardrails (no RPC calls needed). They are designed
 * to be instantiated once and shared across the agent's lifecycle.
 */

export type RateLimitResult = {
  decision: "ALLOW" | "BLOCK";
  reason: string;
  allowExecute: boolean;
};

export interface RateLimiterConfig {
  /** Maximum number of transactions allowed within the window */
  maxTransactions: number;
  /** Time window in seconds (e.g. 3600 for 1 hour) */
  windowSeconds: number;
}

export interface SpendingCapConfig {
  /** Maximum cumulative value allowed within the window (in wei or smallest token unit) */
  maxSpend: bigint;
  /** Time window in seconds */
  windowSeconds: number;
}

/**
 * In-memory transaction rate limiter.
 * Tracks timestamps of recent transactions within a sliding window.
 */
export class RateLimiter {
  private readonly maxTx: number;
  private readonly windowMs: number;
  private timestamps: number[] = [];

  constructor(config: RateLimiterConfig) {
    if (typeof config.maxTransactions !== "number" || config.maxTransactions <= 0) {
      throw new Error("maxTransactions must be a positive integer");
    }
    if (typeof config.windowSeconds !== "number" || config.windowSeconds <= 0) {
      throw new Error("windowSeconds must be a positive number");
    }
    this.maxTx = config.maxTransactions;
    this.windowMs = config.windowSeconds * 1000;
  }

  /**
   * Call this BEFORE each transaction. Returns ALLOW if under the rate limit,
   * BLOCK if the agent has exceeded the transaction cap for this window.
   */
  check(): RateLimitResult {
    const now = Date.now();
    const cutoff = now - this.windowMs;

    // Prune expired timestamps
    this.timestamps = this.timestamps.filter((t) => t > cutoff);

    if (this.timestamps.length >= this.maxTx) {
      return {
        decision: "BLOCK",
        reason: `RATE LIMIT EXCEEDED: ${this.timestamps.length}/${this.maxTx} transactions in the last ${this.windowMs / 1000}s window. — BLOCK`,
        allowExecute: false,
      };
    }

    return {
      decision: "ALLOW",
      reason: `rate limit ok (${this.timestamps.length}/${this.maxTx})`,
      allowExecute: true,
    };
  }

  /**
   * Call this AFTER a transaction is successfully submitted to record it.
   */
  record(): void {
    this.timestamps.push(Date.now());
  }

  /** Returns how many transactions remain in the current window */
  remaining(): number {
    const cutoff = Date.now() - this.windowMs;
    this.timestamps = this.timestamps.filter((t) => t > cutoff);
    return Math.max(0, this.maxTx - this.timestamps.length);
  }
}

/**
 * In-memory spending cap enforcer.
 * Tracks cumulative spend within a sliding window.
 */
export class SpendingCap {
  private readonly maxSpend: bigint;
  private readonly windowMs: number;
  private ledger: Array<{ timestamp: number; amount: bigint }> = [];

  constructor(config: SpendingCapConfig) {
    if (typeof config.maxSpend !== "bigint" || config.maxSpend <= 0n) {
      throw new Error("maxSpend must be a positive bigint");
    }
    if (typeof config.windowSeconds !== "number" || config.windowSeconds <= 0) {
      throw new Error("windowSeconds must be a positive number");
    }
    this.maxSpend = config.maxSpend;
    this.windowMs = config.windowSeconds * 1000;
  }

  /**
   * Check if the proposed spend would exceed the cap.
   * @param proposedAmount The value of the transaction the agent wants to execute (in wei)
   */
  check(proposedAmount: bigint): RateLimitResult {
    if (typeof proposedAmount !== "bigint" || proposedAmount < 0n) {
      return {
        decision: "BLOCK",
        reason: "invalid proposedAmount — BLOCK",
        allowExecute: false,
      };
    }

    const now = Date.now();
    const cutoff = now - this.windowMs;
    this.ledger = this.ledger.filter((e) => e.timestamp > cutoff);

    const currentSpend = this.ledger.reduce((sum, e) => sum + e.amount, 0n);
    const projectedSpend = currentSpend + proposedAmount;

    if (projectedSpend > this.maxSpend) {
      return {
        decision: "BLOCK",
        reason: `SPENDING CAP EXCEEDED: projected spend ${projectedSpend} exceeds cap ${this.maxSpend} in the last ${this.windowMs / 1000}s window. — BLOCK`,
        allowExecute: false,
      };
    }

    return {
      decision: "ALLOW",
      reason: `spending cap ok (${currentSpend}/${this.maxSpend})`,
      allowExecute: true,
    };
  }

  /**
   * Record a successfully submitted transaction amount.
   */
  record(amount: bigint): void {
    this.ledger.push({ timestamp: Date.now(), amount });
  }

  /** Returns the remaining budget in the current window */
  remaining(): bigint {
    const cutoff = Date.now() - this.windowMs;
    this.ledger = this.ledger.filter((e) => e.timestamp > cutoff);
    const spent = this.ledger.reduce((sum, e) => sum + e.amount, 0n);
    const rem = this.maxSpend - spent;
    return rem > 0n ? rem : 0n;
  }
}
