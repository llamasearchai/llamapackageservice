"""
Advanced Collaboration and Workflow Automation Hub
"""
import asyncio
import json
from typing import Dict, List, Any, Optional, Set, Callable
from datetime import datetime, timedelta
from dataclasses import dataclass, field
from pathlib import Path
import uuid
from enum import Enum
import yaml
import aioredis
import websockets
from collections import defaultdict
import networkx as nx
import logging

logger = logging.getLogger(__name__)


class WorkflowStatus(Enum):
    """Workflow execution status"""
    PENDING = "pending"
    RUNNING = "running"
    SUCCESS = "success"
    FAILED = "failed"
    CANCELLED = "cancelled"
    PAUSED = "paused"


class ActionType(Enum):
    """Workflow action types"""
    CODE_ANALYSIS = "code_analysis"
    SECURITY_SCAN = "security_scan"
    DEPENDENCY_UPDATE = "dependency_update"
    TEST_EXECUTION = "test_execution"
    BUILD = "build"
    DEPLOY = "deploy"
    NOTIFICATION = "notification"
    APPROVAL = "approval"
    CUSTOM_SCRIPT = "custom_script"
    AI_ANALYSIS = "ai_analysis"


@dataclass
class WorkflowStep:
    """Individual workflow step"""
    id: str
    name: str
    action: ActionType
    params: Dict[str, Any]
    dependencies: List[str] = field(default_factory=list)
    conditions: Dict[str, Any] = field(default_factory=dict)
    retry_policy: Dict[str, Any] = field(default_factory=dict)
    timeout: int = 300  # seconds
    status: WorkflowStatus = WorkflowStatus.PENDING
    result: Optional[Any] = None
    error: Optional[str] = None
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None


@dataclass
class Workflow:
    """Complete workflow definition"""
    id: str
    name: str
    description: str
    triggers: List[Dict[str, Any]]
    steps: List[WorkflowStep]
    variables: Dict[str, Any] = field(default_factory=dict)
    notifications: List[Dict[str, Any]] = field(default_factory=list)
    created_by: str = "system"
    created_at: datetime = field(default_factory=datetime.utcnow)
    status: WorkflowStatus = WorkflowStatus.PENDING
    execution_id: Optional[str] = None


@dataclass
class CollaborationSession:
    """Real-time collaboration session"""
    id: str
    name: str
    repository: str
    participants: Set[str]
    created_by: str
    created_at: datetime
    active: bool = True
    shared_state: Dict[str, Any] = field(default_factory=dict)
    chat_history: List[Dict[str, Any]] = field(default_factory=list)
    annotations: List[Dict[str, Any]] = field(default_factory=list)
    
    def add_participant(self, user_id: str):
        """Add participant to session"""
        self.participants.add(user_id)
    
    def remove_participant(self, user_id: str):
        """Remove participant from session"""
        self.participants.discard(user_id)
        if not self.participants:
            self.active = False


class WorkflowEngine:
    """Workflow execution engine"""
    
    def __init__(self, redis_url: str = "redis://localhost:6379"):
        self.redis_url = redis_url
        self.redis = None
        self.workflows = {}
        self.executors = {}
        self.running_workflows = {}
        self._register_default_executors()
    
    async def initialize(self):
        """Initialize workflow engine"""
        self.redis = await aioredis.create_redis_pool(self.redis_url)
        await self._load_workflows()
    
    def _register_default_executors(self):
        """Register default action executors"""
        self.executors = {
            ActionType.CODE_ANALYSIS: self._execute_code_analysis,
            ActionType.SECURITY_SCAN: self._execute_security_scan,
            ActionType.DEPENDENCY_UPDATE: self._execute_dependency_update,
            ActionType.TEST_EXECUTION: self._execute_test,
            ActionType.BUILD: self._execute_build,
            ActionType.NOTIFICATION: self._execute_notification,
            ActionType.AI_ANALYSIS: self._execute_ai_analysis,
            ActionType.CUSTOM_SCRIPT: self._execute_custom_script
        }
    
    async def _load_workflows(self):
        """Load workflow definitions"""
        # Load from configuration or database
        workflow_dir = Path("workflows")
        if workflow_dir.exists():
            for workflow_file in workflow_dir.glob("*.yaml"):
                try:
                    with open(workflow_file, 'r') as f:
                        workflow_data = yaml.safe_load(f)
                        workflow = self._parse_workflow(workflow_data)
                        self.workflows[workflow.id] = workflow
                except Exception as e:
                    logger.error(f"Error loading workflow {workflow_file}: {e}")
    
    def _parse_workflow(self, data: Dict) -> Workflow:
        """Parse workflow from configuration"""
        steps = []
        for step_data in data.get('steps', []):
            step = WorkflowStep(
                id=step_data.get('id', str(uuid.uuid4())),
                name=step_data['name'],
                action=ActionType(step_data['action']),
                params=step_data.get('params', {}),
                dependencies=step_data.get('depends_on', []),
                conditions=step_data.get('conditions', {}),
                retry_policy=step_data.get('retry', {}),
                timeout=step_data.get('timeout', 300)
            )
            steps.append(step)
        
        return Workflow(
            id=data.get('id', str(uuid.uuid4())),
            name=data['name'],
            description=data.get('description', ''),
            triggers=data.get('triggers', []),
            steps=steps,
            variables=data.get('variables', {}),
            notifications=data.get('notifications', [])
        )
    
    async def execute_workflow(self, workflow_id: str, context: Dict[str, Any]) -> str:
        """Execute a workflow"""
        if workflow_id not in self.workflows:
            raise ValueError(f"Workflow {workflow_id} not found")
        
        workflow = self.workflows[workflow_id]
        execution_id = str(uuid.uuid4())
        workflow.execution_id = execution_id
        workflow.status = WorkflowStatus.RUNNING
        
        # Store execution state
        self.running_workflows[execution_id] = {
            'workflow': workflow,
            'context': context,
            'started_at': datetime.utcnow()
        }
        
        # Execute workflow asynchronously
        asyncio.create_task(self._run_workflow(workflow, context))
        
        return execution_id
    
    async def _run_workflow(self, workflow: Workflow, context: Dict[str, Any]):
        """Run workflow execution"""
        try:
            # Build execution graph
            graph = self._build_execution_graph(workflow.steps)
            
            # Execute steps in topological order
            for step_id in nx.topological_sort(graph):
                step = next(s for s in workflow.steps if s.id == step_id)
                
                # Check conditions
                if not await self._check_conditions(step, context):
                    step.status = WorkflowStatus.CANCELLED
                    continue
                
                # Execute step
                await self._execute_step(step, context)
                
                # Check if we should continue
                if step.status == WorkflowStatus.FAILED:
                    if not step.retry_policy.get('continue_on_failure', False):
                        workflow.status = WorkflowStatus.FAILED
                        break
            
            # Set final status
            if workflow.status == WorkflowStatus.RUNNING:
                workflow.status = WorkflowStatus.SUCCESS
            
            # Send notifications
            await self._send_workflow_notifications(workflow, context)
            
        except Exception as e:
            logger.error(f"Workflow execution error: {e}")
            workflow.status = WorkflowStatus.FAILED
            workflow.error = str(e)
        
        finally:
            # Cleanup
            if workflow.execution_id in self.running_workflows:
                del self.running_workflows[workflow.execution_id]
    
    def _build_execution_graph(self, steps: List[WorkflowStep]) -> nx.DiGraph:
        """Build directed graph for execution order"""
        graph = nx.DiGraph()
        
        for step in steps:
            graph.add_node(step.id)
            for dep in step.dependencies:
                graph.add_edge(dep, step.id)
        
        # Check for cycles
        if not nx.is_directed_acyclic_graph(graph):
            raise ValueError("Workflow contains circular dependencies")
        
        return graph
    
    async def _check_conditions(self, step: WorkflowStep, context: Dict) -> bool:
        """Check if step conditions are met"""
        for condition_type, condition_value in step.conditions.items():
            if condition_type == 'if':
                # Simple expression evaluation (in production, use safe eval)
                try:
                    result = eval(condition_value, {'context': context})
                    if not result:
                        return False
                except:
                    logger.error(f"Error evaluating condition: {condition_value}")
                    return False
            
            elif condition_type == 'unless':
                try:
                    result = eval(condition_value, {'context': context})
                    if result:
                        return False
                except:
                    return True
        
        return True
    
    async def _execute_step(self, step: WorkflowStep, context: Dict):
        """Execute individual workflow step"""
        step.status = WorkflowStatus.RUNNING
        step.started_at = datetime.utcnow()
        
        max_retries = step.retry_policy.get('max_retries', 0)
        retry_delay = step.retry_policy.get('delay', 5)
        
        for attempt in range(max_retries + 1):
            try:
                # Get executor
                executor = self.executors.get(step.action)
                if not executor:
                    raise ValueError(f"No executor for action {step.action}")
                
                # Execute with timeout
                step.result = await asyncio.wait_for(
                    executor(step.params, context),
                    timeout=step.timeout
                )
                
                step.status = WorkflowStatus.SUCCESS
                break
                
            except asyncio.TimeoutError:
                step.error = f"Step timed out after {step.timeout} seconds"
                step.status = WorkflowStatus.FAILED
                
            except Exception as e:
                step.error = str(e)
                step.status = WorkflowStatus.FAILED
                
                if attempt < max_retries:
                    logger.warning(f"Step {step.name} failed, retrying in {retry_delay}s")
                    await asyncio.sleep(retry_delay)
                    retry_delay *= 2  # Exponential backoff
        
        step.completed_at = datetime.utcnow()
    
    async def _execute_code_analysis(self, params: Dict, context: Dict) -> Dict:
        """Execute code analysis action"""
        # Integration with analytics engine
        repo_path = params.get('repository_path')
        analysis_type = params.get('analysis_type', 'full')
        
        # Placeholder for actual implementation
        return {
            'status': 'completed',
            'metrics': {
                'code_quality': 85,
                'test_coverage': 78,
                'security_score': 92
            }
        }
    
    async def _execute_security_scan(self, params: Dict, context: Dict) -> Dict:
        """Execute security scan action"""
        # Integration with security scanner
        return {
            'status': 'completed',
            'vulnerabilities': [],
            'score': 95
        }
    
    async def _execute_dependency_update(self, params: Dict, context: Dict) -> Dict:
        """Execute dependency update action"""
        # Update dependencies based on params
        return {
            'status': 'completed',
            'updated_dependencies': []
        }
    
    async def _execute_test(self, params: Dict, context: Dict) -> Dict:
        """Execute test suite"""
        test_command = params.get('command', 'pytest')
        
        # Run tests
        import subprocess
        result = subprocess.run(
            test_command.split(),
            capture_output=True,
            text=True
        )
        
        return {
            'status': 'success' if result.returncode == 0 else 'failed',
            'output': result.stdout,
            'errors': result.stderr,
            'exit_code': result.returncode
        }
    
    async def _execute_build(self, params: Dict, context: Dict) -> Dict:
        """Execute build action"""
        build_command = params.get('command', 'make build')
        
        # Run build
        import subprocess
        result = subprocess.run(
            build_command.split(),
            capture_output=True,
            text=True
        )
        
        return {
            'status': 'success' if result.returncode == 0 else 'failed',
            'artifacts': params.get('artifacts', []),
            'exit_code': result.returncode
        }
    
    async def _execute_notification(self, params: Dict, context: Dict) -> Dict:
        """Send notification"""
        notification_type = params.get('type', 'email')
        recipients = params.get('recipients', [])
        message = params.get('message', '')
        
        # Format message with context
        formatted_message = message.format(**context)
        
        # Send notification (placeholder)
        logger.info(f"Sending {notification_type} to {recipients}: {formatted_message}")
        
        return {
            'status': 'sent',
            'recipients': recipients
        }
    
    async def _execute_ai_analysis(self, params: Dict, context: Dict) -> Dict:
        """Execute AI-powered analysis"""
        analysis_prompt = params.get('prompt', '')
        target = params.get('target', '')
        
        # Use Ollama for analysis
        # Placeholder for actual implementation
        return {
            'status': 'completed',
            'analysis': 'AI analysis results',
            'recommendations': []
        }
    
    async def _execute_custom_script(self, params: Dict, context: Dict) -> Dict:
        """Execute custom script"""
        script_path = params.get('script')
        script_args = params.get('args', [])
        
        if not script_path:
            raise ValueError("Script path required")
        
        # Security check - ensure script is in allowed directory
        allowed_dir = Path("scripts/allowed")
        script_path = allowed_dir / script_path
        
        if not script_path.exists():
            raise ValueError(f"Script not found: {script_path}")
        
        # Execute script
        import subprocess
        result = subprocess.run(
            ['python', str(script_path)] + script_args,
            capture_output=True,
            text=True
        )
        
        return {
            'status': 'success' if result.returncode == 0 else 'failed',
            'output': result.stdout,
            'exit_code': result.returncode
        }
    
    async def _send_workflow_notifications(self, workflow: Workflow, context: Dict):
        """Send workflow completion notifications"""
        for notification in workflow.notifications:
            if notification.get('on') == workflow.status.value:
                await self._execute_notification(notification, context)
    
    async def get_workflow_status(self, execution_id: str) -> Dict:
        """Get workflow execution status"""
        if execution_id in self.running_workflows:
            execution = self.running_workflows[execution_id]
            workflow = execution['workflow']
            
            return {
                'execution_id': execution_id,
                'workflow_name': workflow.name,
                'status': workflow.status.value,
                'started_at': execution['started_at'].isoformat(),
                'steps': [
                    {
                        'id': step.id,
                        'name': step.name,
                        'status': step.status.value,
                        'error': step.error
                    }
                    for step in workflow.steps
                ]
            }
        
        # Check completed workflows in Redis
        workflow_data = await self.redis.get(f"workflow:{execution_id}")
        if workflow_data:
            return json.loads(workflow_data)
        
        return None
    
    async def cancel_workflow(self, execution_id: str) -> bool:
        """Cancel running workflow"""
        if execution_id in self.running_workflows:
            workflow = self.running_workflows[execution_id]['workflow']
            workflow.status = WorkflowStatus.CANCELLED
            return True
        return False


class CollaborationManager:
    """Manage real-time collaboration sessions"""
    
    def __init__(self):
        self.sessions: Dict[str, CollaborationSession] = {}
        self.user_sessions: Dict[str, Set[str]] = defaultdict(set)
        self.websocket_connections: Dict[str, websockets.WebSocketServerProtocol] = {}
    
    async def create_session(self, name: str, repository: str, 
                           created_by: str) -> CollaborationSession:
        """Create new collaboration session"""
        session = CollaborationSession(
            id=str(uuid.uuid4()),
            name=name,
            repository=repository,
            participants={created_by},
            created_by=created_by,
            created_at=datetime.utcnow()
        )
        
        self.sessions[session.id] = session
        self.user_sessions[created_by].add(session.id)
        
        # Notify participants
        await self._broadcast_session_update(session.id, {
            'type': 'session_created',
            'session': self._serialize_session(session)
        })
        
        return session
    
    async def join_session(self, session_id: str, user_id: str) -> bool:
        """Join collaboration session"""
        if session_id not in self.sessions:
            return False
        
        session = self.sessions[session_id]
        session.add_participant(user_id)
        self.user_sessions[user_id].add(session_id)
        
        # Notify all participants
        await self._broadcast_session_update(session_id, {
            'type': 'user_joined',
            'user_id': user_id,
            'participants': list(session.participants)
        })
        
        return True
    
    async def leave_session(self, session_id: str, user_id: str):
        """Leave collaboration session"""
        if session_id not in self.sessions:
            return
        
        session = self.sessions[session_id]
        session.remove_participant(user_id)
        self.user_sessions[user_id].discard(session_id)
        
        # Notify remaining participants
        await self._broadcast_session_update(session_id, {
            'type': 'user_left',
            'user_id': user_id,
            'participants': list(session.participants)
        })
        
        # Close session if no participants
        if not session.active:
            await self.close_session(session_id)
    
    async def close_session(self, session_id: str):
        """Close collaboration session"""
        if session_id not in self.sessions:
            return
        
        session = self.sessions[session_id]
        
        # Notify all participants
        await self._broadcast_session_update(session_id, {
            'type': 'session_closed'
        })
        
        # Clean up
        for user_id in session.participants:
            self.user_sessions[user_id].discard(session_id)
        
        del self.sessions[session_id]
    
    async def update_shared_state(self, session_id: str, user_id: str, 
                                 update: Dict[str, Any]):
        """Update shared session state"""
        if session_id not in self.sessions:
            return
        
        session = self.sessions[session_id]
        if user_id not in session.participants:
            return
        
        # Apply update
        session.shared_state.update(update)
        
        # Broadcast to all participants
        await self._broadcast_session_update(session_id, {
            'type': 'state_update',
            'user_id': user_id,
            'update': update,
            'timestamp': datetime.utcnow().isoformat()
        })
    
    async def add_annotation(self, session_id: str, user_id: str, 
                           annotation: Dict[str, Any]):
        """Add annotation to session"""
        if session_id not in self.sessions:
            return
        
        session = self.sessions[session_id]
        if user_id not in session.participants:
            return
        
        # Add annotation
        annotation_data = {
            'id': str(uuid.uuid4()),
            'user_id': user_id,
            'timestamp': datetime.utcnow().isoformat(),
            **annotation
        }
        
        session.annotations.append(annotation_data)
        
        # Broadcast to all participants
        await self._broadcast_session_update(session_id, {
            'type': 'annotation_added',
            'annotation': annotation_data
        })
    
    async def send_chat_message(self, session_id: str, user_id: str, message: str):
        """Send chat message in session"""
        if session_id not in self.sessions:
            return
        
        session = self.sessions[session_id]
        if user_id not in session.participants:
            return
        
        # Add to chat history
        chat_message = {
            'id': str(uuid.uuid4()),
            'user_id': user_id,
            'message': message,
            'timestamp': datetime.utcnow().isoformat()
        }
        
        session.chat_history.append(chat_message)
        
        # Keep only last 1000 messages
        if len(session.chat_history) > 1000:
            session.chat_history = session.chat_history[-1000:]
        
        # Broadcast to all participants
        await self._broadcast_session_update(session_id, {
            'type': 'chat_message',
            'message': chat_message
        })
    
    async def register_websocket(self, user_id: str, websocket: websockets.WebSocketServerProtocol):
        """Register WebSocket connection for user"""
        self.websocket_connections[user_id] = websocket
    
    async def unregister_websocket(self, user_id: str):
        """Unregister WebSocket connection"""
        if user_id in self.websocket_connections:
            del self.websocket_connections[user_id]
        
        # Leave all sessions
        for session_id in list(self.user_sessions[user_id]):
            await self.leave_session(session_id, user_id)
    
    async def _broadcast_session_update(self, session_id: str, update: Dict[str, Any]):
        """Broadcast update to all session participants"""
        if session_id not in self.sessions:
            return
        
        session = self.sessions[session_id]
        message = json.dumps({
            'session_id': session_id,
            **update
        })
        
        # Send to all participants with active WebSocket connections
        for user_id in session.participants:
            if user_id in self.websocket_connections:
                try:
                    await self.websocket_connections[user_id].send(message)
                except Exception as e:
                    logger.error(f"Error sending to {user_id}: {e}")
    
    def _serialize_session(self, session: CollaborationSession) -> Dict:
        """Serialize session for transmission"""
        return {
            'id': session.id,
            'name': session.name,
            'repository': session.repository,
            'participants': list(session.participants),
            'created_by': session.created_by,
            'created_at': session.created_at.isoformat(),
            'active': session.active
        }
    
    def get_user_sessions(self, user_id: str) -> List[Dict]:
        """Get all sessions for a user"""
        sessions = []
        for session_id in self.user_sessions.get(user_id, []):
            if session_id in self.sessions:
                sessions.append(self._serialize_session(self.sessions[session_id]))
        return sessions


class ReviewWorkflow:
    """Code review workflow automation"""
    
    def __init__(self, github_client, ai_analyzer):
        self.github = github_client
        self.ai = ai_analyzer
        self.review_rules = self._load_review_rules()
    
    def _load_review_rules(self) -> Dict[str, Any]:
        """Load code review rules"""
        return {
            'auto_approve_threshold': 0.9,
            'require_tests': True,
            'require_documentation': True,
            'max_file_changes': 50,
            'protected_files': ['*.env', 'secrets/*', 'config/production/*'],
            'mandatory_reviewers': {
                'security': ['security-team'],
                'database': ['db-team'],
                'api': ['api-team']
            }
        }
    
    async def automate_pr_review(self, repo_name: str, pr_number: int) -> Dict:
        """Automated PR review process"""
        pr = self.github.get_repo(repo_name).get_pull(pr_number)
        
        review_result = {
            'pr_number': pr_number,
            'automated_checks': [],
            'ai_analysis': None,
            'suggested_reviewers': [],
            'auto_approved': False,
            'comments': []
        }
        
        # Run automated checks
        checks = await self._run_automated_checks(pr)
        review_result['automated_checks'] = checks
        
        # AI analysis
        ai_review = await self._ai_code_review(pr)
        review_result['ai_analysis'] = ai_review
        
        # Determine reviewers
        suggested_reviewers = await self._suggest_reviewers(pr)
        review_result['suggested_reviewers'] = suggested_reviewers
        
        # Check auto-approval eligibility
        if self._can_auto_approve(checks, ai_review):
            review_result['auto_approved'] = True
            await self._approve_pr(pr)
        else:
            # Post review comments
            await self._post_review_comments(pr, review_result)
        
        return review_result
    
    async def _run_automated_checks(self, pr) -> List[Dict]:
        """Run automated PR checks"""
        checks = []
        
        # Check file count
        if pr.changed_files > self.review_rules['max_file_changes']:
            checks.append({
                'name': 'file_count',
                'passed': False,
                'message': f'Too many files changed ({pr.changed_files})'
            })
        
        # Check for tests
        files = list(pr.get_files())
        has_tests = any('test' in f.filename.lower() for f in files)
        
        if self.review_rules['require_tests'] and not has_tests:
            checks.append({
                'name': 'tests_required',
                'passed': False,
                'message': 'No test files found'
            })
        
        # Check protected files
        for file in files:
            for pattern in self.review_rules['protected_files']:
                if self._matches_pattern(file.filename, pattern):
                    checks.append({
                        'name': 'protected_file',
                        'passed': False,
                        'message': f'Protected file modified: {file.filename}'
                    })
        
        return checks
    
    async def _ai_code_review(self, pr) -> Dict:
        """AI-powered code review"""
        # Get diff
        diff_content = []
        for file in pr.get_files():
            if file.patch:
                diff_content.append(f"File: {file.filename}\n{file.patch}")
        
        # Analyze with AI
        analysis = await self.ai.analyze_code_diff('\n'.join(diff_content))
        
        return {
            'quality_score': analysis.get('quality_score', 0),
            'issues': analysis.get('issues', []),
            'suggestions': analysis.get('suggestions', [])
        }
    
    async def _suggest_reviewers(self, pr) -> List[str]:
        """Suggest appropriate reviewers"""
        reviewers = set()
        
        # Check files for team assignments
        for file in pr.get_files():
            for keyword, team in self.review_rules['mandatory_reviewers'].items():
                if keyword in file.filename.lower():
                    reviewers.update(team)
        
        # Get contributors to modified files
        # This would analyze git history
        
        return list(reviewers)
    
    def _can_auto_approve(self, checks: List[Dict], ai_review: Dict) -> bool:
        """Check if PR can be auto-approved"""
        # All checks must pass
        if any(not check.get('passed', True) for check in checks):
            return False
        
        # AI score must be high enough
        if ai_review.get('quality_score', 0) < self.review_rules['auto_approve_threshold']:
            return False
        
        # No critical issues
        if any(issue.get('severity') == 'critical' for issue in ai_review.get('issues', [])):
            return False
        
        return True
    
    async def _approve_pr(self, pr):
        """Approve pull request"""
        pr.create_review(
            body="Automated review passed all checks ✅",
            event="APPROVE"
        )
    
    async def _post_review_comments(self, pr, review_result: Dict):
        """Post review comments on PR"""
        body = "## Automated Review Results\n\n"
        
        # Automated checks
        if review_result['automated_checks']:
            body += "### Automated Checks\n"
            for check in review_result['automated_checks']:
                status = "✅" if check.get('passed', True) else "❌"
                body += f"- {status} {check['name']}: {check.get('message', 'OK')}\n"
            body += "\n"
        
        # AI Analysis
        if review_result['ai_analysis']:
            ai = review_result['ai_analysis']
            body += f"### AI Analysis\n"
            body += f"Quality Score: {ai.get('quality_score', 0):.1%}\n\n"
            
            if ai.get('issues'):
                body += "**Issues Found:**\n"
                for issue in ai['issues']:
                    body += f"- {issue}\n"
                body += "\n"
            
            if ai.get('suggestions'):
                body += "**Suggestions:**\n"
                for suggestion in ai['suggestions']:
                    body += f"- {suggestion}\n"
        
        # Suggested reviewers
        if review_result['suggested_reviewers']:
            body += f"\n### Suggested Reviewers\n"
            body += f"Consider requesting review from: {', '.join(review_result['suggested_reviewers'])}\n"
        
        pr.create_issue_comment(body)
    
    def _matches_pattern(self, filename: str, pattern: str) -> bool:
        """Check if filename matches pattern"""
        import fnmatch
        return fnmatch.fnmatch(filename, pattern)