"""Security framework for operation validation and sandboxing."""
import asyncio
import hashlib
import json
import os
import re
from dataclasses import dataclass, field
from datetime import datetime, timedelta
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple
import logging

import aiofiles
from pydantic import BaseModel, Field

from ..interfaces import Action, ActionType, ISecurityValidator


logger = logging.getLogger(__name__)


class SecurityLevel(Enum):
    """Security levels for operations."""
    LOW = "low"
    MEDIUM = "medium"
    HIGH = "high"
    CRITICAL = "critical"


class RuleType(Enum):
    """Types of security rules."""
    BLACKLIST = "blacklist"
    WHITELIST = "whitelist"
    PATTERN = "pattern"
    PERMISSION = "permission"


@dataclass
class SecurityRule:
    """Security rule definition."""
    id: str
    type: RuleType
    level: SecurityLevel
    pattern: Optional[str] = None
    actions: List[ActionType] = field(default_factory=list)
    targets: List[str] = field(default_factory=list)
    conditions: Dict[str, Any] = field(default_factory=dict)
    message: str = ""


@dataclass
class SecurityContext:
    """Context for security decisions."""
    sandbox_mode: bool = False
    user_permissions: Set[str] = field(default_factory=set)
    session_id: str = ""
    audit_log: List[Dict[str, Any]] = field(default_factory=list)


class SecurityConfig(BaseModel):
    """Security configuration."""
    enable_sandbox: bool = Field(default=True, description="Enable sandbox mode by default")
    require_confirmation: List[str] = Field(
        default_factory=lambda: ["system_commands", "file_deletion", "network_requests"],
        description="Actions requiring user confirmation"
    )
    blacklisted_commands: List[str] = Field(
        default_factory=lambda: ["rm -rf /", "format", "del /s /q"],
        description="Blacklisted system commands"
    )
    sensitive_patterns: List[str] = Field(
        default_factory=lambda: [
            r"password\s*=",
            r"api[_-]?key\s*=",
            r"secret\s*=",
            r"token\s*=",
            r"private[_-]?key"
        ],
        description="Patterns indicating sensitive data"
    )
    max_file_size: int = Field(default=100_000_000, description="Maximum file size in bytes")
    allowed_domains: List[str] = Field(
        default_factory=lambda: ["github.com", "gitlab.com", "bitbucket.org"],
        description="Allowed domains for network requests"
    )


class SandboxContext:
    """Context manager for sandbox mode."""
    
    def __init__(self, guardian: "SecurityGuardian"):
        self.guardian = guardian
        self.original_mode = None
        
    async def __aenter__(self):
        self.original_mode = self.guardian.context.sandbox_mode
        self.guardian.context.sandbox_mode = True
        logger.info("Entering sandbox mode")
        return self
        
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        self.guardian.context.sandbox_mode = self.original_mode
        logger.info("Exiting sandbox mode")


class SecurityGuardian(ISecurityValidator):
    """Main security validation and enforcement system."""
    
    def __init__(self, config: Optional[SecurityConfig] = None):
        self.config = config or SecurityConfig()
        self.rules: List[SecurityRule] = []
        self.context = SecurityContext(sandbox_mode=self.config.enable_sandbox)
        self._confirmation_cache: Dict[str, Tuple[bool, datetime]] = {}
        self._load_default_rules()
        
    def _load_default_rules(self):
        """Load default security rules."""
        # System command blacklist
        self.rules.append(SecurityRule(
            id="sys_cmd_blacklist",
            type=RuleType.BLACKLIST,
            level=SecurityLevel.CRITICAL,
            actions=[ActionType.EXECUTE],
            pattern="|".join(re.escape(cmd) for cmd in self.config.blacklisted_commands),
            message="Dangerous system command detected"
        ))
        
        # Sensitive data patterns
        for pattern in self.config.sensitive_patterns:
            self.rules.append(SecurityRule(
                id=f"sensitive_pattern_{hashlib.md5(pattern.encode()).hexdigest()[:8]}",
                type=RuleType.PATTERN,
                level=SecurityLevel.HIGH,
                pattern=pattern,
                message="Potential sensitive data exposure"
            ))
        
        # File operations
        self.rules.append(SecurityRule(
            id="file_size_limit",
            type=RuleType.PERMISSION,
            level=SecurityLevel.MEDIUM,
            actions=[ActionType.TYPE],
            conditions={"max_size": self.config.max_file_size},
            message="File size exceeds limit"
        ))
        
    async def validate_action(self, action: Action) -> Tuple[bool, Optional[str]]:
        """Validate an action against security rules."""
        # Log action for audit
        await self._audit_log(action, "validation_started")
        
        # Check sandbox mode restrictions
        if self.context.sandbox_mode:
            if action.type in [ActionType.EXECUTE, ActionType.GITHUB]:
                return False, "Action not allowed in sandbox mode"
        
        # Check against all rules
        for rule in self.rules:
            violation = await self._check_rule(action, rule)
            if violation:
                await self._audit_log(action, "validation_failed", {"rule": rule.id})
                return False, f"{rule.message}: {violation}"
        
        # Check if confirmation required
        if await self._requires_confirmation(action):
            confirmed = await self.request_permission(action)
            if not confirmed:
                await self._audit_log(action, "user_denied")
                return False, "User denied permission"
        
        await self._audit_log(action, "validation_passed")
        return True, None
        
    async def _check_rule(self, action: Action, rule: SecurityRule) -> Optional[str]:
        """Check a single rule against an action."""
        # Check action type
        if rule.actions and action.type not in rule.actions:
            return None
            
        if rule.type == RuleType.BLACKLIST:
            if rule.pattern and action.value:
                if re.search(rule.pattern, str(action.value), re.IGNORECASE):
                    return "Blacklisted pattern matched"
                    
        elif rule.type == RuleType.PATTERN:
            if rule.pattern:
                # Check in action value and metadata
                to_check = [str(action.value)] if action.value else []
                to_check.extend(str(v) for v in action.metadata.values())
                
                for text in to_check:
                    if re.search(rule.pattern, text, re.IGNORECASE):
                        return f"Sensitive pattern detected: {rule.pattern}"
                        
        elif rule.type == RuleType.PERMISSION:
            if not await self._check_permission(action, rule):
                return "Permission check failed"
                
        return None
        
    async def _check_permission(self, action: Action, rule: SecurityRule) -> bool:
        """Check permission-based rules."""
        if "max_size" in rule.conditions and action.metadata.get("size"):
            if action.metadata["size"] > rule.conditions["max_size"]:
                return False
                
        if "allowed_domains" in rule.conditions and action.metadata.get("domain"):
            if action.metadata["domain"] not in rule.conditions["allowed_domains"]:
                return False
                
        return True
        
    async def _requires_confirmation(self, action: Action) -> bool:
        """Check if action requires user confirmation."""
        action_category = self._categorize_action(action)
        return action_category in self.config.require_confirmation
        
    def _categorize_action(self, action: Action) -> str:
        """Categorize action for confirmation requirements."""
        if action.type == ActionType.EXECUTE:
            return "system_commands"
        elif action.type == ActionType.TYPE and action.metadata.get("file_operation") == "delete":
            return "file_deletion"
        elif action.type == ActionType.GITHUB and action.metadata.get("network_request"):
            return "network_requests"
        return "general"
        
    async def is_sandbox_mode(self) -> bool:
        """Check if running in sandbox mode."""
        return self.context.sandbox_mode
        
    async def request_permission(self, action: Action) -> bool:
        """Request user permission for an action."""
        # Check cache first
        cache_key = f"{action.type}:{action.target}:{action.value}"
        if cache_key in self._confirmation_cache:
            cached_result, cache_time = self._confirmation_cache[cache_key]
            if datetime.utcnow() - cache_time < timedelta(minutes=5):
                return cached_result
        
        # In production, this would show a UI prompt
        # For now, log and return based on config
        logger.warning(f"Permission requested for action: {action}")
        
        # Simulate user response based on sandbox mode
        result = not self.context.sandbox_mode
        
        # Cache the result
        self._confirmation_cache[cache_key] = (result, datetime.utcnow())
        
        return result
        
    def sandbox_mode(self) -> SandboxContext:
        """Get sandbox context manager."""
        return SandboxContext(self)
        
    async def add_rule(self, rule: SecurityRule):
        """Add a custom security rule."""
        self.rules.append(rule)
        logger.info(f"Added security rule: {rule.id}")
        
    async def remove_rule(self, rule_id: str):
        """Remove a security rule."""
        self.rules = [r for r in self.rules if r.id != rule_id]
        logger.info(f"Removed security rule: {rule_id}")
        
    async def grant_permission(self, permission: str):
        """Grant a permission to the current session."""
        self.context.user_permissions.add(permission)
        await self._audit_log(None, "permission_granted", {"permission": permission})
        
    async def revoke_permission(self, permission: str):
        """Revoke a permission from the current session."""
        self.context.user_permissions.discard(permission)
        await self._audit_log(None, "permission_revoked", {"permission": permission})
        
    async def _audit_log(self, action: Optional[Action], event: str, details: Optional[Dict] = None):
        """Log security events for audit."""
        log_entry = {
            "timestamp": datetime.utcnow().isoformat(),
            "event": event,
            "session_id": self.context.session_id,
            "sandbox_mode": self.context.sandbox_mode,
        }
        
        if action:
            log_entry["action"] = {
                "id": action.id,
                "type": action.type.value,
                "target": str(action.target) if action.target else None,
            }
            
        if details:
            log_entry["details"] = details
            
        self.context.audit_log.append(log_entry)
        
        # Persist audit log periodically
        if len(self.context.audit_log) >= 100:
            await self._persist_audit_log()
            
    async def _persist_audit_log(self):
        """Persist audit log to file."""
        log_dir = Path("logs/security")
        log_dir.mkdir(parents=True, exist_ok=True)
        
        filename = f"audit_{datetime.utcnow().strftime('%Y%m%d_%H%M%S')}.json"
        log_path = log_dir / filename
        
        async with aiofiles.open(log_path, 'w') as f:
            await f.write(json.dumps(self.context.audit_log, indent=2))
            
        # Clear in-memory log
        self.context.audit_log = []
        
    async def export_rules(self) -> Dict[str, Any]:
        """Export security rules for backup/sharing."""
        return {
            "version": "1.0",
            "exported_at": datetime.utcnow().isoformat(),
            "rules": [
                {
                    "id": rule.id,
                    "type": rule.type.value,
                    "level": rule.level.value,
                    "pattern": rule.pattern,
                    "actions": [a.value for a in rule.actions],
                    "targets": rule.targets,
                    "conditions": rule.conditions,
                    "message": rule.message
                }
                for rule in self.rules
            ]
        }
        
    async def import_rules(self, rules_data: Dict[str, Any]):
        """Import security rules."""
        for rule_data in rules_data.get("rules", []):
            rule = SecurityRule(
                id=rule_data["id"],
                type=RuleType(rule_data["type"]),
                level=SecurityLevel(rule_data["level"]),
                pattern=rule_data.get("pattern"),
                actions=[ActionType(a) for a in rule_data.get("actions", [])],
                targets=rule_data.get("targets", []),
                conditions=rule_data.get("conditions", {}),
                message=rule_data.get("message", "")
            )
            await self.add_rule(rule)