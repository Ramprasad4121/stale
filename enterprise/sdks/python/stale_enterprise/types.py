from dataclasses import dataclass
from typing import Optional, List, Dict, Any
from datetime import datetime

@dataclass
class GuardResult:
    guard_name: str
    check_type: str
    decision: str  # ALLOW or BLOCK
    reason: str
    duration_ms: float
    metadata: Optional[Dict[str, Any]] = None

@dataclass
class PipelineResponse:
    trace_id: str
    decision: str
    blocked_by: Optional[str]
    reason: str
    duration_ms: float
    results: List[GuardResult]
    policy_id: Optional[str]
    policy_version: Optional[int]
    audit_log_id: str

    @property
    def is_allow(self) -> bool:
        return self.decision == "ALLOW"
    
    @property
    def is_block(self) -> bool:
        return self.decision == "BLOCK"

@dataclass
class AuditLog:
    id: str
    trace_id: str
    decision: str
    blocked_by: Optional[str]
    reason: str
    duration_ms: float
    environment: str
    created_at: str
    request: Dict[str, Any]
    metadata: Dict[str, Any]
