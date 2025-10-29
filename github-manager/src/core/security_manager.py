"""
Advanced Security Manager for GitHub Repository Management
"""
import os
import json
import base64
import hashlib
import secrets
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple
from datetime import datetime, timedelta
from cryptography.fernet import Fernet
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.kdf.pbkdf2 import PBKDF2HMAC
import jwt
import bcrypt
from dataclasses import dataclass
import aioredis
import asyncio
from functools import wraps
import logging

logger = logging.getLogger(__name__)


@dataclass
class SecurityPolicy:
    """Security policy configuration"""
    min_password_length: int = 12
    require_mfa: bool = True
    token_expiry_hours: int = 24
    max_login_attempts: int = 5
    lockout_duration_minutes: int = 30
    require_api_key_rotation_days: int = 90
    allowed_ip_ranges: List[str] = None
    audit_retention_days: int = 365


@dataclass
class AuditEntry:
    """Security audit log entry"""
    timestamp: datetime
    user_id: str
    action: str
    resource: str
    ip_address: str
    user_agent: str
    success: bool
    details: Dict[str, Any]


class SecureTokenStorage:
    """Secure storage for API tokens and secrets"""
    
    def __init__(self, master_key_path: str = "~/.llamasearch/master.key"):
        self.master_key_path = Path(master_key_path).expanduser()
        self._cipher = self._initialize_cipher()
        self._token_store = {}
        
    def _initialize_cipher(self) -> Fernet:
        """Initialize or create master encryption key"""
        if self.master_key_path.exists():
            with open(self.master_key_path, 'rb') as f:
                key = f.read()
        else:
            # Generate new master key
            key = Fernet.generate_key()
            self.master_key_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Secure file permissions
            with open(self.master_key_path, 'wb') as f:
                f.write(key)
            os.chmod(self.master_key_path, 0o600)
            
        return Fernet(key)
    
    def store_token(self, service: str, token: str, metadata: Dict = None):
        """Securely store an API token"""
        encrypted_token = self._cipher.encrypt(token.encode())
        
        self._token_store[service] = {
            'token': base64.b64encode(encrypted_token).decode(),
            'stored_at': datetime.utcnow().isoformat(),
            'metadata': metadata or {},
            'checksum': hashlib.sha256(token.encode()).hexdigest()
        }
        
        self._persist_tokens()
    
    def retrieve_token(self, service: str) -> Optional[str]:
        """Retrieve and decrypt a token"""
        if service not in self._token_store:
            return None
        
        try:
            encrypted_token = base64.b64decode(self._token_store[service]['token'])
            decrypted = self._cipher.decrypt(encrypted_token)
            return decrypted.decode()
        except Exception as e:
            logger.error(f"Failed to decrypt token for {service}: {e}")
            return None
    
    def rotate_token(self, service: str, new_token: str):
        """Rotate an existing token"""
        old_token_data = self._token_store.get(service)
        
        if old_token_data:
            # Archive old token
            if 'history' not in old_token_data:
                old_token_data['history'] = []
            
            old_token_data['history'].append({
                'rotated_at': datetime.utcnow().isoformat(),
                'checksum': old_token_data['checksum']
            })
        
        self.store_token(service, new_token, old_token_data.get('metadata', {}))
    
    def _persist_tokens(self):
        """Persist encrypted tokens to secure storage"""
        storage_path = self.master_key_path.parent / 'tokens.enc'
        
        # Encrypt entire token store
        data = json.dumps(self._token_store).encode()
        encrypted = self._cipher.encrypt(data)
        
        with open(storage_path, 'wb') as f:
            f.write(encrypted)
        
        os.chmod(storage_path, 0o600)


class AuthenticationManager:
    """Handle authentication and authorization"""
    
    def __init__(self, redis_url: str = "redis://localhost:6379"):
        self.redis_url = redis_url
        self.redis = None
        self.jwt_secret = os.getenv('JWT_SECRET', secrets.token_urlsafe(32))
        self.policy = SecurityPolicy()
        
    async def initialize(self):
        """Initialize Redis connection"""
        self.redis = await aioredis.create_redis_pool(self.redis_url)
    
    async def create_user(self, username: str, password: str, email: str, 
                         roles: List[str] = None) -> Dict:
        """Create a new user account"""
        # Validate password strength
        if not self._validate_password_strength(password):
            raise ValueError("Password does not meet security requirements")
        
        # Hash password
        salt = bcrypt.gensalt()
        hashed = bcrypt.hashpw(password.encode(), salt)
        
        user_data = {
            'username': username,
            'email': email,
            'password_hash': hashed.decode(),
            'roles': roles or ['user'],
            'created_at': datetime.utcnow().isoformat(),
            'mfa_enabled': False,
            'api_keys': [],
            'locked': False
        }
        
        # Store in Redis
        await self.redis.setex(
            f"user:{username}",
            3600 * 24 * 365,  # 1 year
            json.dumps(user_data)
        )
        
        return {'username': username, 'roles': user_data['roles']}
    
    async def authenticate(self, username: str, password: str, 
                          mfa_code: Optional[str] = None) -> Optional[str]:
        """Authenticate user and return JWT token"""
        # Check login attempts
        attempts_key = f"login_attempts:{username}"
        attempts = await self.redis.get(attempts_key)
        
        if attempts and int(attempts) >= self.policy.max_login_attempts:
            raise Exception("Account locked due to too many failed attempts")
        
        # Get user data
        user_data = await self.redis.get(f"user:{username}")
        if not user_data:
            await self._increment_login_attempts(username)
            return None
        
        user = json.loads(user_data)
        
        # Check if account is locked
        if user.get('locked'):
            raise Exception("Account is locked")
        
        # Verify password
        if not bcrypt.checkpw(password.encode(), user['password_hash'].encode()):
            await self._increment_login_attempts(username)
            return None
        
        # Verify MFA if enabled
        if user.get('mfa_enabled') and self.policy.require_mfa:
            if not mfa_code or not await self._verify_mfa(username, mfa_code):
                return None
        
        # Reset login attempts
        await self.redis.delete(attempts_key)
        
        # Generate JWT token
        token = self._generate_jwt(username, user['roles'])
        
        # Store session
        await self._create_session(username, token)
        
        return token
    
    def _validate_password_strength(self, password: str) -> bool:
        """Validate password meets security requirements"""
        if len(password) < self.policy.min_password_length:
            return False
        
        # Check complexity
        has_upper = any(c.isupper() for c in password)
        has_lower = any(c.islower() for c in password)
        has_digit = any(c.isdigit() for c in password)
        has_special = any(c in '!@#$%^&*()-_=+[]{}|;:,.<>?' for c in password)
        
        return all([has_upper, has_lower, has_digit, has_special])
    
    def _generate_jwt(self, username: str, roles: List[str]) -> str:
        """Generate JWT token"""
        payload = {
            'username': username,
            'roles': roles,
            'exp': datetime.utcnow() + timedelta(hours=self.policy.token_expiry_hours),
            'iat': datetime.utcnow(),
            'jti': secrets.token_urlsafe(16)
        }
        
        return jwt.encode(payload, self.jwt_secret, algorithm='HS256')
    
    async def _increment_login_attempts(self, username: str):
        """Increment failed login attempts"""
        key = f"login_attempts:{username}"
        await self.redis.incr(key)
        await self.redis.expire(key, self.policy.lockout_duration_minutes * 60)
    
    async def _create_session(self, username: str, token: str):
        """Create user session"""
        session_data = {
            'username': username,
            'token': token,
            'created_at': datetime.utcnow().isoformat(),
            'last_activity': datetime.utcnow().isoformat()
        }
        
        await self.redis.setex(
            f"session:{token}",
            self.policy.token_expiry_hours * 3600,
            json.dumps(session_data)
        )
    
    async def verify_token(self, token: str) -> Optional[Dict]:
        """Verify JWT token"""
        try:
            payload = jwt.decode(token, self.jwt_secret, algorithms=['HS256'])
            
            # Check if session exists
            session = await self.redis.get(f"session:{token}")
            if not session:
                return None
            
            # Update last activity
            session_data = json.loads(session)
            session_data['last_activity'] = datetime.utcnow().isoformat()
            await self.redis.setex(
                f"session:{token}",
                self.policy.token_expiry_hours * 3600,
                json.dumps(session_data)
            )
            
            return payload
            
        except jwt.ExpiredSignatureError:
            return None
        except jwt.InvalidTokenError:
            return None
    
    async def create_api_key(self, username: str, key_name: str, 
                           permissions: List[str]) -> str:
        """Create API key for user"""
        # Generate secure API key
        api_key = f"llama_{secrets.token_urlsafe(32)}"
        
        key_data = {
            'key': hashlib.sha256(api_key.encode()).hexdigest(),
            'name': key_name,
            'permissions': permissions,
            'created_at': datetime.utcnow().isoformat(),
            'last_used': None,
            'expires_at': (datetime.utcnow() + 
                          timedelta(days=self.policy.require_api_key_rotation_days)).isoformat()
        }
        
        # Store key data
        await self.redis.setex(
            f"api_key:{key_data['key']}",
            self.policy.require_api_key_rotation_days * 24 * 3600,
            json.dumps(key_data)
        )
        
        # Update user's API keys
        user_data = await self.redis.get(f"user:{username}")
        if user_data:
            user = json.loads(user_data)
            user['api_keys'].append(key_data['key'])
            await self.redis.setex(
                f"user:{username}",
                3600 * 24 * 365,
                json.dumps(user)
            )
        
        return api_key
    
    async def verify_api_key(self, api_key: str) -> Optional[Dict]:
        """Verify API key"""
        key_hash = hashlib.sha256(api_key.encode()).hexdigest()
        key_data = await self.redis.get(f"api_key:{key_hash}")
        
        if not key_data:
            return None
        
        data = json.loads(key_data)
        
        # Check expiration
        if datetime.fromisoformat(data['expires_at']) < datetime.utcnow():
            return None
        
        # Update last used
        data['last_used'] = datetime.utcnow().isoformat()
        await self.redis.setex(
            f"api_key:{key_hash}",
            self.policy.require_api_key_rotation_days * 24 * 3600,
            json.dumps(data)
        )
        
        return data


class AuditLogger:
    """Security audit logging"""
    
    def __init__(self, log_path: str = "~/.llamasearch/audit"):
        self.log_path = Path(log_path).expanduser()
        self.log_path.mkdir(parents=True, exist_ok=True)
        self.current_log = None
        self._rotate_log()
    
    def _rotate_log(self):
        """Rotate audit log daily"""
        date_str = datetime.utcnow().strftime("%Y%m%d")
        self.current_log = self.log_path / f"audit_{date_str}.jsonl"
    
    async def log_event(self, event: AuditEntry):
        """Log security event"""
        # Rotate log if needed
        if datetime.utcnow().date() != self.current_log.stat().st_mtime:
            self._rotate_log()
        
        # Write event
        event_data = {
            'timestamp': event.timestamp.isoformat(),
            'user_id': event.user_id,
            'action': event.action,
            'resource': event.resource,
            'ip_address': event.ip_address,
            'user_agent': event.user_agent,
            'success': event.success,
            'details': event.details
        }
        
        async with aiofiles.open(self.current_log, 'a') as f:
            await f.write(json.dumps(event_data) + '\n')
    
    async def search_events(self, filters: Dict) -> List[AuditEntry]:
        """Search audit logs"""
        events = []
        
        # Search through log files
        for log_file in sorted(self.log_path.glob("audit_*.jsonl"), reverse=True):
            async with aiofiles.open(log_file, 'r') as f:
                async for line in f:
                    event_data = json.loads(line)
                    
                    # Apply filters
                    if self._matches_filters(event_data, filters):
                        events.append(AuditEntry(**event_data))
                    
                    if len(events) >= filters.get('limit', 1000):
                        return events
        
        return events
    
    def _matches_filters(self, event: Dict, filters: Dict) -> bool:
        """Check if event matches filters"""
        for key, value in filters.items():
            if key == 'start_date':
                if datetime.fromisoformat(event['timestamp']) < value:
                    return False
            elif key == 'end_date':
                if datetime.fromisoformat(event['timestamp']) > value:
                    return False
            elif key in event and event[key] != value:
                return False
        
        return True


class RBACManager:
    """Role-Based Access Control"""
    
    def __init__(self):
        self.roles = {
            'admin': {
                'permissions': ['*'],
                'description': 'Full system access'
            },
            'developer': {
                'permissions': [
                    'repo:read', 'repo:write', 'repo:analyze',
                    'api:read', 'api:write'
                ],
                'description': 'Developer access'
            },
            'analyst': {
                'permissions': [
                    'repo:read', 'repo:analyze',
                    'report:read', 'report:write'
                ],
                'description': 'Analyst access'
            },
            'viewer': {
                'permissions': [
                    'repo:read', 'report:read'
                ],
                'description': 'Read-only access'
            }
        }
    
    def check_permission(self, user_roles: List[str], required_permission: str) -> bool:
        """Check if user has required permission"""
        for role in user_roles:
            if role not in self.roles:
                continue
            
            role_perms = self.roles[role]['permissions']
            
            # Check for wildcard
            if '*' in role_perms:
                return True
            
            # Check specific permission
            if required_permission in role_perms:
                return True
            
            # Check wildcard permissions
            perm_parts = required_permission.split(':')
            for i in range(len(perm_parts)):
                wildcard_perm = ':'.join(perm_parts[:i+1] + ['*'])
                if wildcard_perm in role_perms:
                    return True
        
        return False
    
    def get_user_permissions(self, user_roles: List[str]) -> Set[str]:
        """Get all permissions for user roles"""
        permissions = set()
        
        for role in user_roles:
            if role in self.roles:
                permissions.update(self.roles[role]['permissions'])
        
        return permissions


def require_auth(permission: str = None):
    """Decorator for authentication and authorization"""
    def decorator(func):
        @wraps(func)
        async def wrapper(self, *args, **kwargs):
            # Get token from request context
            token = kwargs.get('auth_token')
            if not token:
                raise Exception("Authentication required")
            
            # Verify token
            auth_manager = getattr(self, 'auth_manager', None)
            if not auth_manager:
                raise Exception("Authentication not configured")
            
            user_data = await auth_manager.verify_token(token)
            if not user_data:
                raise Exception("Invalid or expired token")
            
            # Check permission if specified
            if permission:
                rbac = RBACManager()
                if not rbac.check_permission(user_data['roles'], permission):
                    raise Exception(f"Permission denied: {permission}")
            
            # Add user context
            kwargs['user'] = user_data
            
            # Log audit event
            if hasattr(self, 'audit_logger'):
                await self.audit_logger.log_event(AuditEntry(
                    timestamp=datetime.utcnow(),
                    user_id=user_data['username'],
                    action=f"{func.__name__}",
                    resource=str(args),
                    ip_address=kwargs.get('ip_address', 'unknown'),
                    user_agent=kwargs.get('user_agent', 'unknown'),
                    success=True,
                    details={'permission': permission}
                ))
            
            return await func(self, *args, **kwargs)
        
        return wrapper
    return decorator


class SecurityScanner:
    """Security vulnerability scanner"""
    
    def __init__(self):
        self.vulnerability_db = self._load_vulnerability_db()
        self.security_rules = self._load_security_rules()
    
    def _load_vulnerability_db(self) -> Dict:
        """Load known vulnerabilities"""
        return {
            'python': {
                'eval': {'severity': 'critical', 'description': 'Code injection risk'},
                'exec': {'severity': 'critical', 'description': 'Code injection risk'},
                'pickle.loads': {'severity': 'high', 'description': 'Deserialization vulnerability'},
                'subprocess.call': {'severity': 'medium', 'description': 'Command injection risk'},
            },
            'javascript': {
                'eval': {'severity': 'critical', 'description': 'Code injection risk'},
                'innerHTML': {'severity': 'high', 'description': 'XSS vulnerability'},
                'document.write': {'severity': 'high', 'description': 'XSS vulnerability'},
            }
        }
    
    def _load_security_rules(self) -> List[Dict]:
        """Load security scanning rules"""
        return [
            {
                'pattern': r'(password|secret|key)\s*=\s*["\'][^"\']+["\']',
                'severity': 'critical',
                'description': 'Hardcoded credentials'
            },
            {
                'pattern': r'https?://[^/\s]+:[^@\s]+@',
                'severity': 'critical',
                'description': 'Credentials in URL'
            },
            {
                'pattern': r'-----BEGIN (RSA |EC )?PRIVATE KEY-----',
                'severity': 'critical',
                'description': 'Private key exposure'
            }
        ]
    
    async def scan_repository(self, repo_path: Path) -> Dict[str, List[Dict]]:
        """Scan repository for security vulnerabilities"""
        vulnerabilities = {
            'critical': [],
            'high': [],
            'medium': [],
            'low': []
        }
        
        for file_path in repo_path.rglob('*'):
            if file_path.is_file() and file_path.suffix in ['.py', '.js', '.ts', '.java']:
                try:
                    with open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = f.read()
                        
                    # Scan for vulnerabilities
                    file_vulns = await self._scan_file(file_path, content)
                    
                    for vuln in file_vulns:
                        vulnerabilities[vuln['severity']].append(vuln)
                        
                except Exception as e:
                    logger.error(f"Error scanning {file_path}: {e}")
        
        return vulnerabilities
    
    async def _scan_file(self, file_path: Path, content: str) -> List[Dict]:
        """Scan individual file for vulnerabilities"""
        vulnerabilities = []
        
        # Check for known vulnerable patterns
        language = self._detect_language(file_path)
        if language in self.vulnerability_db:
            for pattern, info in self.vulnerability_db[language].items():
                if pattern in content:
                    vulnerabilities.append({
                        'file': str(file_path),
                        'type': 'vulnerable_function',
                        'pattern': pattern,
                        'severity': info['severity'],
                        'description': info['description'],
                        'line': self._find_line_number(content, pattern)
                    })
        
        # Check security rules
        for rule in self.security_rules:
            import re
            matches = re.finditer(rule['pattern'], content)
            for match in matches:
                vulnerabilities.append({
                    'file': str(file_path),
                    'type': 'security_rule',
                    'pattern': rule['pattern'],
                    'severity': rule['severity'],
                    'description': rule['description'],
                    'line': content[:match.start()].count('\n') + 1,
                    'match': match.group(0)[:50] + '...' if len(match.group(0)) > 50 else match.group(0)
                })
        
        return vulnerabilities
    
    def _detect_language(self, file_path: Path) -> str:
        """Detect programming language from file extension"""
        ext_map = {
            '.py': 'python',
            '.js': 'javascript',
            '.ts': 'javascript',
            '.java': 'java',
            '.go': 'go',
            '.rs': 'rust'
        }
        return ext_map.get(file_path.suffix, 'unknown')
    
    def _find_line_number(self, content: str, pattern: str) -> int:
        """Find line number of pattern in content"""
        lines = content.split('\n')
        for i, line in enumerate(lines, 1):
            if pattern in line:
                return i
        return 0