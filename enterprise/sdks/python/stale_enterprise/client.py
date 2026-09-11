import requests
from typing import Optional, List, Dict, Any
from .types import PipelineResponse, GuardResult, AuditLog

class StaleError(Exception):
    def __init__(self, message: str, status: int = None, code: str = None, trace_id: str = None):
        super().__init__(message)
        self.status = status
        self.code = code
        self.trace_id = trace_id

class Stale:
    """
    Stale Enterprise Client - Fail-closed guardrails
    
    Args:
        api_key: Your Stale API key (stale_live_...)
        base_url: API base URL (default: https://api.stale.sh)
        environment: production, staging, or development
        timeout: Request timeout in seconds
    """
    
    def __init__(
        self,
        api_key: str,
        base_url: str = "https://api.stale.sh",
        environment: str = "production",
        timeout: int = 10
    ):
        if not api_key:
            raise ValueError("api_key required - get from https://app.stale.sh/api-keys")
        self.api_key = api_key
        self.base_url = base_url.rstrip("/")
        self.environment = environment
        self.timeout = timeout
        self.session = requests.Session()
        self.session.headers.update({
            "Authorization": f"Bearer {api_key}",
            "Content-Type": "application/json",
            "X-Stale-SDK": "python/1.0.0",
            "X-Stale-Environment": environment,
        })

    def _request(self, method: str, path: str, **kwargs) -> Dict[str, Any]:
        url = f"{self.base_url}{path}"
        try:
            resp = self.session.request(method, url, timeout=self.timeout, **kwargs)
            if not resp.ok:
                try:
                    err = resp.json()
                    msg = err.get("error") or err.get("message") or f"HTTP {resp.status_code}"
                except:
                    msg = f"HTTP {resp.status_code}: {resp.text[:200]}"
                raise StaleError(msg, status=resp.status_code, trace_id=err.get("trace_id") if 'err' in locals() else None)
            return resp.json()
        except requests.Timeout:
            # Fail-closed: timeout → BLOCK
            raise StaleError(f"Timeout after {self.timeout}s - BLOCK (fail closed)", status=408, code="TIMEOUT")
        except requests.RequestException as e:
            raise StaleError(f"Network error - BLOCK (fail closed): {e}", code="NETWORK_ERROR")

    def _parse_pipeline_response(self, data: Dict) -> PipelineResponse:
        results = [
            GuardResult(
                guard_name=r.get("guard_name", ""),
                check_type=r.get("check_type", ""),
                decision=r.get("decision", ""),
                reason=r.get("reason", ""),
                duration_ms=r.get("duration_ms", 0),
                metadata=r.get("metadata")
            )
            for r in data.get("results", [])
        ]
        return PipelineResponse(
            trace_id=data.get("trace_id", ""),
            decision=data.get("decision", ""),
            blocked_by=data.get("blocked_by"),
            reason=data.get("reason", ""),
            duration_ms=data.get("duration_ms", 0),
            results=results,
            policy_id=data.get("policy_id"),
            policy_version=data.get("policy_version"),
            audit_log_id=data.get("audit_log_id", "")
        )

    class Check:
        def __init__(self, parent: "Stale"):
            self._parent = parent

        def price(self, rpc_url: str, feed: str = "ETH/USD", max_age_seconds: int = 60, amount_eth: float = None):
            """Check Chainlink price feed freshness"""
            return self._parent._request("POST", "/v1/check/price", json={
                "rpc_url": rpc_url,
                "feed": feed,
                "max_age_seconds": max_age_seconds,
                "amount_eth": amount_eth
            })

        def gas(self, rpc_url: str, max_gas_gwei: int = 50):
            """Check gas price"""
            return self._parent._request("POST", "/v1/check/gas", json={
                "rpc_url": rpc_url,
                "max_gas_gwei": max_gas_gwei
            })

        def sequencer(self, rpc_url: str, chain_id: int = 42161):
            """Check L2 sequencer uptime"""
            return self._parent._request("POST", "/v1/check/sequencer", json={
                "rpc_url": rpc_url,
                "chain_id": chain_id
            })

        def mev(self, rpc_url: str):
            """Check MEV protection"""
            return self._parent._request("POST", "/v1/check/mev", json={
                "rpc_url": rpc_url
            })

    class Pipeline:
        def __init__(self, parent: "Stale"):
            self._parent = parent

        def run(
            self,
            rpc_url: str,
            policy_id: str = None,
            environment: str = None,
            chain_id: int = None,
            checks: List[Dict] = None,
            context: Dict = None
        ) -> PipelineResponse:
            """
            Run full guardrail pipeline
            
            Args:
                rpc_url: Your RPC endpoint (should be MEV-protected like Flashbots)
                policy_id: Policy ID or name (uses default if omitted)
                chain_id: Expected chain ID
                checks: Custom checks (overrides policy if provided)
                context: Transaction context (target_address, amount_usd, etc)
            
            Returns:
                PipelineResponse with decision ALLOW or BLOCK
            
            Example:
                result = client.pipeline.run(
                    rpc_url="https://rpc.flashbots.net",
                    chain_id=1,
                    context={"target_address": "0xE592...", "amount_usd": 50000}
                )
                if result.is_block:
                    print(f"BLOCKED by {result.blocked_by}")
            """
            data = self._parent._request("POST", "/v1/pipeline/run", json={
                "rpc_url": rpc_url,
                "policy_id": policy_id,
                "environment": environment or self._parent.environment,
                "chain_id": chain_id,
                "checks": checks or [],
                "context": context or {}
            })
            response = self._parent._parse_pipeline_response(data)
            
            if response.is_block:
                print(f"[stale] BLOCKED by {response.blocked_by}: {response.reason} (trace: {response.trace_id})")
            
            return response

        def assert_allow(
            self,
            rpc_url: str,
            **kwargs
        ) -> PipelineResponse:
            """Run pipeline and raise StaleError if BLOCKED (fail-closed helper)"""
            result = self.run(rpc_url, **kwargs)
            if result.is_block:
                raise StaleError(
                    f"BLOCKED by {result.blocked_by}: {result.reason}",
                    status=403,
                    code="GUARDRAIL_BLOCKED",
                    trace_id=result.trace_id
                )
            return result

    class Audit:
        def __init__(self, parent: "Stale"):
            self._parent = parent

        def logs(self, decision: str = None, blocked_by: str = None, environment: str = None, limit: int = 100, search: str = None):
            params = {}
            if decision: params["decision"] = decision
            if blocked_by: params["blocked_by"] = blocked_by
            if environment: params["environment"] = environment
            if limit: params["limit"] = limit
            if search: params["search"] = search
            return self._parent._request("GET", "/v1/audit/logs", params=params)

        def stats(self):
            """Get audit stats: total_checks, block_rate, gas_saved, etc"""
            return self._parent._request("GET", "/v1/audit/stats")

    @property
    def check(self):
        return self.Check(self)

    @property
    def pipeline(self):
        return self.Pipeline(self)

    @property
    def audit(self):
        return self.Audit(self)

    def feeds(self):
        """List available Chainlink feeds"""
        return self._request("GET", "/v1/feeds")
