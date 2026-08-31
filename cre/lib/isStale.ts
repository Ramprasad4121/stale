// keep in sync with src/isStale.ts
export type Decision = "ALLOW" | "BLOCK";

export type IsStaleInput = {
  updatedAt: bigint | number | string | null | undefined;
  nowSeconds: number;
  maxAgeSeconds: number;
};

export type IsStaleResult = {
  decision: Decision;
  ageSeconds: number | null;
  reason: string;
};

/**
 * Pure freshness check — no RPC. Fail closed on missing, unparseable, or future timestamp.
 * Safe to unit-test and reuse in CRE workflows.
 * Official field: Data Feed `latestRoundData.updatedAt` (uint80 seconds).
 */
export function isStale({
  updatedAt,
  nowSeconds,
  maxAgeSeconds,
}: IsStaleInput): IsStaleResult {
  // Validate window first — bad window is BLOCK (fail closed, no partial age)
  if (!Number.isFinite(nowSeconds) || !Number.isFinite(maxAgeSeconds)) {
    return {
      decision: "BLOCK",
      ageSeconds: null,
      reason: "missing or invalid now/maxAge — BLOCK (fail closed)",
    };
  }
  if (maxAgeSeconds < 0) {
    return {
      decision: "BLOCK",
      ageSeconds: null,
      reason: "maxAgeSeconds must be >= 0 — BLOCK",
    };
  }

  // Parse updatedAt → seconds (fail closed on any unparseable)
  let updatedAtSeconds: number;
  try {
    if (updatedAt === null || updatedAt === undefined) throw new Error("missing");

    if (typeof updatedAt === "bigint") {
      if (updatedAt < 0n) throw new Error("negative");
      // Use Number for age math; updatedAt fits in 53 bits (uint80 but timestamp < 2^31)
      updatedAtSeconds = Number(updatedAt);
    } else if (typeof updatedAt === "number") {
      if (!Number.isFinite(updatedAt)) throw new Error("not finite");
      updatedAtSeconds = Math.trunc(updatedAt);
    } else if (typeof updatedAt === "string") {
      const s = updatedAt.trim();
      if (s === "") throw new Error("empty");
      if (s.startsWith("0x") || s.startsWith("0X")) {
        updatedAtSeconds = Number(BigInt(s));
      } else {
        const n = Number(s);
        if (!Number.isFinite(n)) throw new Error("not finite");
        updatedAtSeconds = Math.trunc(n);
      }
    } else {
      throw new Error("unsupported type");
    }

    if (!Number.isFinite(updatedAtSeconds) || updatedAtSeconds < 0) {
      throw new Error("invalid seconds");
    }
  } catch {
    return {
      decision: "BLOCK",
      ageSeconds: null,
      reason: "missing or unparseable updatedAt — BLOCK (fail closed)",
    };
  }

  const nowTrunc = Math.trunc(nowSeconds);
  const ageSeconds = nowTrunc - updatedAtSeconds;

  // not-yet-valid is BLOCK — clock skew or future timestamp (per AGENTS.md)
  if (ageSeconds < 0) {
    return {
      decision: "BLOCK",
      ageSeconds,
      reason: `not-yet-valid: updatedAt ${updatedAtSeconds} is in the future (now ${nowTrunc}) — BLOCK`,
    };
  }
  if (ageSeconds > maxAgeSeconds) {
    return {
      decision: "BLOCK",
      ageSeconds,
      reason: `stale: age ${ageSeconds}s > maxAge ${maxAgeSeconds}s — BLOCK`,
    };
  }
  return {
    decision: "ALLOW",
    ageSeconds,
    reason: `fresh: age ${ageSeconds}s <= maxAge ${maxAgeSeconds}s — ALLOW`,
  };
}
