"""
Configuration Manager
"""
import os
import yaml
from pathlib import Path
from typing import Any, Dict, Optional


class ConfigManager:
    def __init__(self, config_path: str = "config/github_config.yaml"):
        self.config_path = Path(config_path)
        self.config = self._load_config()
        self._apply_env_overrides()
    
    def _load_config(self) -> Dict[str, Any]:
        """Load configuration from YAML file"""
        if not self.config_path.exists():
            # Create default config
            default_config = {
                'github_token': os.getenv('GITHUB_TOKEN', ''),
                'local_repos_path': '~/Development',
                'backup_path': '~/Backups/repos',
                'mcp_servers': [
                    {'name': 'github', 'host': 'localhost', 'port': 3001},
                    {'name': 'filesystem', 'host': 'localhost', 'port': 3002},
                    {'name': 'project', 'host': 'localhost', 'port': 3003}
                ],
                'ollama_host': 'http://localhost:11434',
                'log_level': 'INFO',
                'web_dashboard': {
                    'host': '0.0.0.0',
                    'port': 8000
                }
            }
            
            # Create config directory
            self.config_path.parent.mkdir(parents=True, exist_ok=True)
            
            # Save default config
            with open(self.config_path, 'w') as f:
                yaml.dump(default_config, f, default_flow_style=False)
            
            return default_config
        
        with open(self.config_path, 'r') as f:
            return yaml.safe_load(f) or {}
    
    def _apply_env_overrides(self):
        """Apply environment variable overrides"""
        env_mappings = {
            'GITHUB_TOKEN': 'github_token',
            'REPOS_PATH': 'local_repos_path',
            'BACKUP_PATH': 'backup_path',
            'OLLAMA_HOST': 'ollama_host',
            'LOG_LEVEL': 'log_level'
        }
        
        for env_var, config_key in env_mappings.items():
            value = os.getenv(env_var)
            if value:
                self.config[config_key] = value
    
    def get(self, key: str, default: Any = None) -> Any:
        """Get configuration value"""
        keys = key.split('.')
        value = self.config
        
        for k in keys:
            if isinstance(value, dict) and k in value:
                value = value[k]
            else:
                return default
        
        return value
    
    def set(self, key: str, value: Any):
        """Set configuration value"""
        keys = key.split('.')
        config = self.config
        
        for k in keys[:-1]:
            if k not in config:
                config[k] = {}
            config = config[k]
        
        config[keys[-1]] = value
    
    def save(self):
        """Save configuration to file"""
        with open(self.config_path, 'w') as f:
            yaml.dump(self.config, f, default_flow_style=False)
    
    def validate(self) -> bool:
        """Validate configuration"""
        required_keys = ['github_token', 'local_repos_path']
        
        for key in required_keys:
            if not self.get(key):
                raise ValueError(f"Required configuration key missing: {key}")
        
        return True