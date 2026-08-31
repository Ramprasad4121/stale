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

// Fail closed: missing, unparseable, or future timestamp => BLOCK.
// No Chainlink call here — pure check so it can be unit tested and reused in CRE.
export function isStale({
  updatedAt,
  nowSeconds,
  maxAgeSeconds,
}: IsStaleInput): IsStaleResult {
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

  let updatedAtSeconds: number | null = null;
  try {
    if (updatedAt === null || updatedAt === undefined || updatedAt === "") {
      throw new Error("missing");
    }
    if (typeof updatedAt === "bigint") {
      if (updatedAt < 0n) throw new Error("negative");
      updatedAtSeconds = Number(updatedAt);
    } else if (typeof updatedAt === "number") {
      if (!Number.isFinite(updatedAt)) throw new Error("not finite");
      updatedAtSeconds = Math.floor(updatedAt);
    } else if (typeof updatedAt === "string") {
      const s = updatedAt.trim();
      if (s === "") throw new Error("empty");
      if (s.startsWith("0x") || s.startsWith("0X")) {
        updatedAtSeconds = Number(BigInt(s));
      } else {
        const n = Number(s);
        if (!Number.isFinite(n)) throw new Error("not finite");
        updatedAtSeconds = Math.floor(n);
      }
    } else {
      throw new Error("unsupported type");
    }
    if (!Number.isFinite(updatedAtSeconds!) || updatedAtSeconds! < 0) {
      throw new Error("invalid seconds");
    }
  } catch {
    return {
      decision: "BLOCK",
      ageSeconds: null,
      reason: "missing or unparseable updatedAt — BLOCK (fail closed)",
    };
  }

  const ageSeconds = Math.floor(nowSeconds) - updatedAtSeconds!;

  // not-yet-valid is also BLOCK — clock skew or future timestamp
  if (ageSeconds < 0) {
    return {
      decision: "BLOCK",
      ageSeconds,
      reason: `not-yet-valid: updatedAt ${updatedAtSeconds} is in the future (now ${Math.floor(nowSeconds)}) — BLOCK`,
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
