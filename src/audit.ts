/**
 * @module audit
 * Structured Audit Logger for compliance and post-mortem analysis.
 *
 * Every guardrail decision (ALLOW / BLOCK) is recorded with a timestamp,
 * the guardrail name, the decision, the reason, and arbitrary metadata.
 * This is critical for regulatory compliance (MiCA, SEC), incident response,
 * and debugging agent behavior in production.
 */

export type AuditEntry = {
  timestamp: string;
  guardrail: string;
  decision: "ALLOW" | "BLOCK";
  reason: string;
  metadata?: Record<string, unknown>;
};

export type AuditLoggerConfig = {
  /** Maximum entries to retain in memory (FIFO). Default: 10000 */
  maxEntries?: number;
  /** Optional callback fired on every new entry (e.g. ship to Datadog, stdout) */
  onEntry?: (entry: AuditEntry) => void;
};

/**
 * In-memory structured audit logger.
 * Instantiate once and pass to all guardrail calls, or use the convenience
 * `pipeline.audit` integration.
 */
export class AuditLogger {
  private entries: AuditEntry[] = [];
  private readonly maxEntries: number;
  private readonly onEntry?: (entry: AuditEntry) => void;

  constructor(config: AuditLoggerConfig = {}) {
    this.maxEntries = config.maxEntries ?? 10_000;
    this.onEntry = config.onEntry;
  }

  /**
   * Record a guardrail decision.
   */
  record(
    guardrail: string,
    decision: "ALLOW" | "BLOCK",
    reason: string,
    metadata?: Record<string, unknown>,
  ): AuditEntry {
    const entry: AuditEntry = {
      timestamp: new Date().toISOString(),
      guardrail,
      decision,
      reason,
      metadata,
    };

    this.entries.push(entry);

    // FIFO eviction
    if (this.entries.length > this.maxEntries) {
      this.entries.shift();
    }

    if (this.onEntry) {
      try {
        this.onEntry(entry);
      } catch {
        // Never let a callback crash the guardrail pipeline
      }
    }

    return entry;
  }

  /** Get all recorded entries (most recent last) */
  getEntries(): ReadonlyArray<AuditEntry> {
    return this.entries;
  }

  /** Get only BLOCK entries */
  getBlocks(): ReadonlyArray<AuditEntry> {
    return this.entries.filter((e) => e.decision === "BLOCK");
  }

  /** Get only entries for a specific guardrail */
  getByGuardrail(name: string): ReadonlyArray<AuditEntry> {
    return this.entries.filter((e) => e.guardrail === name);
  }

  /** Total number of entries */
  size(): number {
    return this.entries.length;
  }

  /** Clear all entries */
  clear(): void {
    this.entries = [];
  }

  /** Export entries as a JSON string (for shipping to external systems) */
  toJSON(): string {
    return JSON.stringify(this.entries);
  }
}
