/**
 * Stale Enterprise TypeScript SDK
 * Fail-closed guardrails for agents that touch money
 * 
 * @example
 * import { Stale } from "@stale-enterprise/sdk";
 * const stale = new Stale({ apiKey: "stale_live_...", environment: "production" });
 * const result = await stale.pipeline.run({ rpc_url: "https://rpc.flashbots.net", chain_id: 1 });
 * if (result.decision === "BLOCK") throw new Error(result.reason);
 */

export interface StaleConfig {
  apiKey: string;
  baseUrl?: string;
  environment?: "production" | "staging" | "development";
  timeout?: number;
  retries?: number;
}

export interface CheckRequest {
  rpc_url: string;
  chain_id?: number;
  config?: Record<string, any>;
}

export interface PipelineCheck {
  type: "price" | "gas" | "gas_1559" | "sequencer" | "mev" | "chain_id" | "is_contract";
  config: Record<string, any>;
  enabled?: boolean;
}

export interface PipelineRequest {
  rpc_url: string;
  policy_id?: string;
  environment?: string;
  chain_id?: number;
  checks?: PipelineCheck[];
  context?: {
    target_address?: string;
    amount_usd?: number;
    token_address?: string;
    from_address?: string;
  };
  metadata?: Record<string, any>;
}

export interface GuardResult {
  guard_name: string;
  check_type: string;
  decision: "ALLOW" | "BLOCK";
  reason: string;
  duration_ms: number;
  metadata?: any;
}

export interface PipelineResponse {
  trace_id: string;
  decision: "ALLOW" | "BLOCK";
  blocked_by: string | null;
  reason: string;
  duration_ms: number;
  results: GuardResult[];
  policy_id: string | null;
  policy_version: number | null;
  audit_log_id: string;
}

export interface PriceCheckRequest {
  rpc_url: string;
  feed?: string;
  max_age_seconds?: number;
  amount_eth?: number;
}

export interface AuditLog {
  id: string;
  trace_id: string;
  decision: "ALLOW" | "BLOCK";
  blocked_by: string | null;
  reason: string;
  duration_ms: number;
  environment: string;
  created_at: string;
}

class StaleError extends Error {
  constructor(
    message: string,
    public status?: number,
    public code?: string,
    public trace_id?: string
  ) {
    super(message);
    this.name = "StaleError";
  }
}

export class Stale {
  private apiKey: string;
  private baseUrl: string;
  private environment: string;
  private timeout: number;

  constructor(config: StaleConfig) {
    if (!config.apiKey) throw new Error("apiKey required - get from https://app.stale.sh/api-keys");
    this.apiKey = config.apiKey;
    this.baseUrl = config.baseUrl || "https://api.stale.sh";
    this.environment = config.environment || "production";
    this.timeout = config.timeout || 10000;
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const res = await fetch(url, {
        ...options,
        signal: controller.signal,
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${this.apiKey}`,
          "X-Stale-SDK": "typescript/1.0.0",
          "X-Stale-Environment": this.environment,
          ...(options.headers || {}),
        },
      });

      clearTimeout(timeoutId);

      if (!res.ok) {
        const errBody = await res.json().catch(() => ({}));
        throw new StaleError(
          errBody.error || errBody.message || `HTTP ${res.status}`,
          res.status,
          errBody.code,
          errBody.trace_id
        );
      }

      return res.json();
    } catch (err: any) {
      clearTimeout(timeoutId);
      if (err.name === "AbortError") {
        throw new StaleError(`Timeout after ${this.timeout}ms - BLOCK (fail closed)`, 408, "TIMEOUT");
      }
      throw err;
    }
  }

  check = {
    price: async (req: PriceCheckRequest) => {
      return this.request<{
        decision: "ALLOW" | "BLOCK";
        reason: string;
        allow_execute: boolean;
        metadata: any;
        trace_id: string;
      }>("/v1/check/price", {
        method: "POST",
        body: JSON.stringify(req),
      });
    },

    gas: async (req: { rpc_url: string; max_gas_gwei?: number }) => {
      return this.request<{
        decision: "ALLOW" | "BLOCK";
        reason: string;
        allow_execute: boolean;
        trace_id: string;
      }>("/v1/check/gas", {
        method: "POST",
        body: JSON.stringify(req),
      });
    },

    sequencer: async (req: { rpc_url: string; chain_id?: number }) => {
      return this.request<{
        decision: "ALLOW" | "BLOCK";
        reason: string;
        allow_execute: boolean;
      }>("/v1/check/sequencer", {
        method: "POST",
        body: JSON.stringify(req),
      });
    },

    mev: async (req: { rpc_url: string }) => {
      return this.request<{
        decision: "ALLOW" | "BLOCK";
        reason: string;
        is_mev_protected: boolean;
      }>("/v1/check/mev", {
        method: "POST",
        body: JSON.stringify(req),
      });
    },
  };

  pipeline = {
    run: async (req: PipelineRequest): Promise<PipelineResponse> => {
      const res = await this.request<PipelineResponse>("/v1/pipeline/run", {
        method: "POST",
        body: JSON.stringify({
          ...req,
          environment: req.environment || this.environment,
        }),
      });

      // Enterprise: fail-closed enforcement
      if (res.decision === "BLOCK") {
        // In production, this would also emit metric and log
        console.warn(`[stale] BLOCKED by ${res.blocked_by}: ${res.reason} (trace: ${res.trace_id})`);
      }

      return res;
    },

    assertAllow: async (req: PipelineRequest): Promise<PipelineResponse> => {
      const result = await this.request<PipelineResponse>("/v1/pipeline/run", {
        method: "POST",
        body: JSON.stringify(req),
      });

      if (result.decision === "BLOCK") {
        throw new StaleError(
          `BLOCKED by ${result.blocked_by}: ${result.reason}`,
          403,
          "GUARDRAIL_BLOCKED",
          result.trace_id
        );
      }

      return result;
    },
  };

  audit = {
    logs: async (query?: {
      decision?: "ALLOW" | "BLOCK";
      blocked_by?: string;
      environment?: string;
      limit?: number;
      search?: string;
    }) => {
      const params = new URLSearchParams();
      if (query) {
        Object.entries(query).forEach(([k, v]) => {
          if (v !== undefined) params.append(k, String(v));
        });
      }
      return this.request<{ logs: AuditLog[]; total: number }>(
        `/v1/audit/logs?${params.toString()}`
      );
    },

    stats: async () => {
      return this.request<{
        total_checks: number;
        total_blocks: number;
        block_rate: number;
        avg_duration_ms: number;
        gas_saved_usd: number;
        top_blocked_reasons: { guard: string; count: number; percentage: number }[];
      }>("/v1/audit/stats");
    },
  };

  policies = {
    list: async (environment?: string) => {
      const q = environment ? `?environment=${environment}` : "";
      return this.request<{ policies: any[]; total: number }>(`/v1/policies${q}`);
    },
  };

  feeds = {
    list: async () => {
      return this.request<{ feeds: any[]; total: number; default_feed: string }>("/v1/feeds");
    },
  };
}

// LangChain integration
export class StaleTool {
  name = "stale_guardrail";
  description = "Check if a transaction is safe to execute. Must be called before any transaction that moves money. Returns ALLOW or BLOCK with reason.";

  constructor(private config: StaleConfig & { policyId?: string }) {}

  async _call(input: string): Promise<string> {
    const stale = new Stale(this.config);
    try {
      const parsed = JSON.parse(input);
      if (!parsed.rpc_url) {
        return `BLOCK: Missing rpc_url in input - BLOCK (fail closed, must provide MEV-protected RPC)`;
      }
      const result = await stale.pipeline.run({
        rpc_url: parsed.rpc_url,
        policy_id: this.config.policyId,
        chain_id: parsed.chain_id || 1,
        context: parsed.context,
      });

      if (result.decision === "BLOCK") {
        return `BLOCKED by ${result.blocked_by}: ${result.reason}. DO NOT PROCEED. Trace: ${result.trace_id}`;
      }

      return `ALLOW: ${result.reason}. Safe to proceed. Trace: ${result.trace_id}, Duration: ${result.duration_ms}ms`;
    } catch (err: any) {
      // Fail-closed: any error → BLOCK
      return `BLOCK: Stale check failed - ${err.message} - FAIL CLOSED, DO NOT PROCEED`;
    }
  }
}

export default Stale;
export { StaleError };
