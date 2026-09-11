"""
Stale Enterprise Python SDK
Fail-closed guardrails for agents that touch money

Example:
    from stale_enterprise import Stale

    client = Stale(api_key="stale_live_...", environment="production")
    result = client.pipeline.run(
        rpc_url="https://rpc.flashbots.net",
        chain_id=1,
        context={"amount_usd": 50000}
    )
    if result.decision == "BLOCK":
        raise Exception(f"Blocked by {result.blocked_by}: {result.reason}")
"""

from .client import Stale, StaleError
from .types import PipelineResponse, GuardResult, AuditLog

__version__ = "1.0.0"
__all__ = ["Stale", "StaleError", "PipelineResponse", "GuardResult", "AuditLog"]
