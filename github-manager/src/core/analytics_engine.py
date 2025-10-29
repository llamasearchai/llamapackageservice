"""
Advanced Analytics and Reporting Engine for GitHub Repository Manager
"""
import asyncio
import json
import pandas as pd
import numpy as np
from datetime import datetime, timedelta
from pathlib import Path
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, asdict
import plotly.graph_objects as go
import plotly.express as px
from plotly.subplots import make_subplots
import networkx as nx
from sklearn.cluster import KMeans
from sklearn.decomposition import PCA
from sklearn.preprocessing import StandardScaler
import seaborn as sns
import matplotlib.pyplot as plt
from collections import defaultdict, Counter
import re
from textblob import TextBlob
import aiofiles
import hashlib
from jinja2 import Template
import logging

logger = logging.getLogger(__name__)


@dataclass
class CodeMetrics:
    """Code quality metrics"""
    lines_of_code: int
    cyclomatic_complexity: float
    maintainability_index: float
    technical_debt_hours: float
    test_coverage: float
    documentation_coverage: float
    code_duplication_ratio: float
    dependency_count: int
    security_score: float
    performance_score: float


@dataclass
class RepositoryInsights:
    """Repository analysis insights"""
    repo_name: str
    primary_language: str
    health_score: float
    activity_level: str
    contributor_diversity: float
    issue_resolution_time: float
    pr_merge_time: float
    code_quality_trend: str
    risk_factors: List[str]
    recommendations: List[str]


@dataclass
class DeveloperMetrics:
    """Developer productivity metrics"""
    username: str
    commits_count: int
    lines_added: int
    lines_removed: int
    pr_count: int
    issue_count: int
    review_count: int
    productivity_score: float
    quality_score: float
    collaboration_score: float


class MetricsCalculator:
    """Calculate various code and repository metrics"""
    
    def __init__(self):
        self.language_weights = {
            'Python': 1.0,
            'JavaScript': 1.1,
            'TypeScript': 0.9,
            'Go': 0.8,
            'Rust': 0.7,
            'Java': 1.2
        }
    
    async def calculate_code_metrics(self, repo_path: Path) -> CodeMetrics:
        """Calculate comprehensive code metrics"""
        metrics = {
            'lines_of_code': 0,
            'cyclomatic_complexity': 0,
            'file_count': 0,
            'test_files': 0,
            'doc_files': 0
        }
        
        # Analyze all files
        for file_path in repo_path.rglob('*'):
            if file_path.is_file() and self._is_code_file(file_path):
                file_metrics = await self._analyze_file(file_path)
                metrics['lines_of_code'] += file_metrics['loc']
                metrics['cyclomatic_complexity'] += file_metrics['complexity']
                metrics['file_count'] += 1
                
                if self._is_test_file(file_path):
                    metrics['test_files'] += 1
                elif self._is_doc_file(file_path):
                    metrics['doc_files'] += 1
        
        # Calculate derived metrics
        avg_complexity = metrics['cyclomatic_complexity'] / max(metrics['file_count'], 1)
        
        # Maintainability Index (MI) = 171 - 5.2 * ln(V) - 0.23 * G - 16.2 * ln(LOC)
        # Simplified version
        mi = max(0, min(100, 171 - 5.2 * np.log(avg_complexity + 1) - 
                        16.2 * np.log(metrics['lines_of_code'] + 1)))
        
        # Technical debt estimation (hours)
        tech_debt = (100 - mi) * metrics['lines_of_code'] / 1000
        
        # Test coverage estimation
        test_coverage = min(100, (metrics['test_files'] / max(metrics['file_count'], 1)) * 200)
        
        # Documentation coverage
        doc_coverage = min(100, (metrics['doc_files'] / max(metrics['file_count'], 1)) * 300)
        
        return CodeMetrics(
            lines_of_code=metrics['lines_of_code'],
            cyclomatic_complexity=avg_complexity,
            maintainability_index=mi,
            technical_debt_hours=tech_debt,
            test_coverage=test_coverage,
            documentation_coverage=doc_coverage,
            code_duplication_ratio=await self._calculate_duplication_ratio(repo_path),
            dependency_count=await self._count_dependencies(repo_path),
            security_score=await self._calculate_security_score(repo_path),
            performance_score=await self._calculate_performance_score(repo_path)
        )
    
    async def _analyze_file(self, file_path: Path) -> Dict[str, Any]:
        """Analyze individual file metrics"""
        metrics = {'loc': 0, 'complexity': 1}
        
        try:
            async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                content = await f.read()
                lines = content.split('\n')
                
                # Count lines of code (excluding empty lines and comments)
                for line in lines:
                    stripped = line.strip()
                    if stripped and not stripped.startswith(('#', '//', '/*', '*')):
                        metrics['loc'] += 1
                
                # Estimate cyclomatic complexity
                # Count decision points
                complexity_patterns = [
                    r'\bif\b', r'\belse\b', r'\belif\b', r'\bfor\b', 
                    r'\bwhile\b', r'\bcase\b', r'\bcatch\b', r'\btry\b'
                ]
                
                for pattern in complexity_patterns:
                    metrics['complexity'] += len(re.findall(pattern, content))
                
        except Exception as e:
            logger.error(f"Error analyzing file {file_path}: {e}")
        
        return metrics
    
    def _is_code_file(self, file_path: Path) -> bool:
        """Check if file is a code file"""
        code_extensions = {
            '.py', '.js', '.ts', '.jsx', '.tsx', '.go', '.rs', '.java', 
            '.cpp', '.c', '.h', '.hpp', '.cs', '.rb', '.php', '.swift'
        }
        return file_path.suffix in code_extensions
    
    def _is_test_file(self, file_path: Path) -> bool:
        """Check if file is a test file"""
        test_patterns = ['test_', '_test', '.test.', '.spec.', 'tests/', 'test/']
        return any(pattern in str(file_path).lower() for pattern in test_patterns)
    
    def _is_doc_file(self, file_path: Path) -> bool:
        """Check if file is documentation"""
        doc_extensions = {'.md', '.rst', '.txt', '.adoc'}
        doc_patterns = ['readme', 'changelog', 'contributing', 'docs/']
        
        return (file_path.suffix in doc_extensions or 
                any(pattern in str(file_path).lower() for pattern in doc_patterns))
    
    async def _calculate_duplication_ratio(self, repo_path: Path) -> float:
        """Calculate code duplication ratio"""
        # Simplified duplication detection
        hash_counts = defaultdict(int)
        total_blocks = 0
        
        for file_path in repo_path.rglob('*'):
            if file_path.is_file() and self._is_code_file(file_path):
                try:
                    async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = await f.read()
                        # Hash code blocks (simplified - every 10 lines)
                        lines = content.split('\n')
                        for i in range(0, len(lines), 10):
                            block = '\n'.join(lines[i:i+10])
                            if block.strip():
                                block_hash = hashlib.md5(block.encode()).hexdigest()
                                hash_counts[block_hash] += 1
                                total_blocks += 1
                except:
                    pass
        
        # Calculate duplication ratio
        duplicated_blocks = sum(1 for count in hash_counts.values() if count > 1)
        return (duplicated_blocks / max(total_blocks, 1)) * 100
    
    async def _count_dependencies(self, repo_path: Path) -> int:
        """Count project dependencies"""
        dep_count = 0
        
        # Check various dependency files
        dep_files = {
            'requirements.txt': self._count_python_deps,
            'package.json': self._count_npm_deps,
            'Cargo.toml': self._count_cargo_deps,
            'go.mod': self._count_go_deps,
            'pom.xml': self._count_maven_deps
        }
        
        for dep_file, counter in dep_files.items():
            file_path = repo_path / dep_file
            if file_path.exists():
                dep_count += await counter(file_path)
        
        return dep_count
    
    async def _count_python_deps(self, file_path: Path) -> int:
        """Count Python dependencies"""
        count = 0
        try:
            async with aiofiles.open(file_path, 'r') as f:
                async for line in f:
                    if line.strip() and not line.startswith('#'):
                        count += 1
        except:
            pass
        return count
    
    async def _count_npm_deps(self, file_path: Path) -> int:
        """Count NPM dependencies"""
        try:
            async with aiofiles.open(file_path, 'r') as f:
                data = json.loads(await f.read())
                count = len(data.get('dependencies', {}))
                count += len(data.get('devDependencies', {}))
                return count
        except:
            return 0
    
    async def _count_cargo_deps(self, file_path: Path) -> int:
        """Count Cargo dependencies"""
        # Simplified - would use toml parser in production
        count = 0
        try:
            async with aiofiles.open(file_path, 'r') as f:
                in_deps = False
                async for line in f:
                    if '[dependencies]' in line:
                        in_deps = True
                    elif line.startswith('[') and in_deps:
                        break
                    elif in_deps and '=' in line:
                        count += 1
        except:
            pass
        return count
    
    async def _count_go_deps(self, file_path: Path) -> int:
        """Count Go dependencies"""
        count = 0
        try:
            async with aiofiles.open(file_path, 'r') as f:
                async for line in f:
                    if line.strip().startswith('require '):
                        count += 1
        except:
            pass
        return count
    
    async def _count_maven_deps(self, file_path: Path) -> int:
        """Count Maven dependencies"""
        # Simplified - would use XML parser in production
        count = 0
        try:
            async with aiofiles.open(file_path, 'r') as f:
                content = await f.read()
                count = content.count('<dependency>')
        except:
            pass
        return count
    
    async def _calculate_security_score(self, repo_path: Path) -> float:
        """Calculate security score (0-100)"""
        score = 100
        
        # Check for security files
        security_files = ['.github/SECURITY.md', 'SECURITY.md', '.github/dependabot.yml']
        for sec_file in security_files:
            if (repo_path / sec_file).exists():
                score += 5
        
        # Check for vulnerable patterns (simplified)
        vulnerable_patterns = [
            'eval(', 'exec(', 'system(', 'shell_exec(',
            'document.write(', 'innerHTML', 'dangerouslySetInnerHTML'
        ]
        
        for file_path in repo_path.rglob('*'):
            if file_path.is_file() and self._is_code_file(file_path):
                try:
                    async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = await f.read()
                        for pattern in vulnerable_patterns:
                            if pattern in content:
                                score -= 5
                                break
                except:
                    pass
        
        return max(0, min(100, score))
    
    async def _calculate_performance_score(self, repo_path: Path) -> float:
        """Calculate performance score (0-100)"""
        score = 80  # Base score
        
        # Check for performance optimizations
        perf_indicators = {
            'async': 5,
            'await': 5,
            'concurrent': 3,
            'parallel': 3,
            'cache': 4,
            'memo': 4,
            'optimize': 2
        }
        
        for file_path in repo_path.rglob('*'):
            if file_path.is_file() and self._is_code_file(file_path):
                try:
                    async with aiofiles.open(file_path, 'r', encoding='utf-8', errors='ignore') as f:
                        content = await f.read().lower()
                        for indicator, points in perf_indicators.items():
                            if indicator in content:
                                score += points
                                break
                except:
                    pass
        
        return min(100, score)


class TrendAnalyzer:
    """Analyze trends and patterns in repository data"""
    
    def __init__(self):
        self.time_periods = {
            'daily': 1,
            'weekly': 7,
            'monthly': 30,
            'quarterly': 90,
            'yearly': 365
        }
    
    def analyze_commit_trends(self, commits: List[Dict]) -> Dict[str, Any]:
        """Analyze commit trends over time"""
        if not commits:
            return {}
        
        # Convert to DataFrame
        df = pd.DataFrame(commits)
        df['date'] = pd.to_datetime(df['date'])
        df.set_index('date', inplace=True)
        
        trends = {}
        
        # Commits per time period
        for period, days in self.time_periods.items():
            resampled = df.resample(f'{days}D').count()
            trends[f'commits_per_{period}'] = resampled['sha'].mean()
        
        # Developer activity
        dev_activity = df.groupby('author').size().sort_values(ascending=False)
        trends['top_contributors'] = dev_activity.head(10).to_dict()
        
        # Time-based patterns
        df['hour'] = df.index.hour
        df['dayofweek'] = df.index.dayofweek
        
        trends['peak_hours'] = df.groupby('hour').size().idxmax()
        trends['peak_day'] = df.groupby('dayofweek').size().idxmax()
        
        # Commit message sentiment
        sentiments = []
        for msg in df.get('message', []):
            if msg:
                blob = TextBlob(str(msg))
                sentiments.append(blob.sentiment.polarity)
        
        trends['avg_sentiment'] = np.mean(sentiments) if sentiments else 0
        
        return trends
    
    def analyze_issue_trends(self, issues: List[Dict]) -> Dict[str, Any]:
        """Analyze issue trends"""
        if not issues:
            return {}
        
        df = pd.DataFrame(issues)
        df['created_at'] = pd.to_datetime(df['created_at'])
        df['closed_at'] = pd.to_datetime(df.get('closed_at'))
        
        trends = {}
        
        # Issue resolution time
        resolved = df[df['closed_at'].notna()].copy()
        resolved['resolution_time'] = (resolved['closed_at'] - resolved['created_at']).dt.days
        
        trends['avg_resolution_days'] = resolved['resolution_time'].mean()
        trends['median_resolution_days'] = resolved['resolution_time'].median()
        
        # Issue categories
        label_counts = defaultdict(int)
        for labels in df.get('labels', []):
            if labels:
                for label in labels:
                    label_counts[label['name']] += 1
        
        trends['top_labels'] = dict(sorted(label_counts.items(), 
                                         key=lambda x: x[1], 
                                         reverse=True)[:10])
        
        # Open vs closed ratio
        trends['open_issues'] = len(df[df['state'] == 'open'])
        trends['closed_issues'] = len(df[df['state'] == 'closed'])
        trends['open_rate'] = trends['open_issues'] / max(len(df), 1)
        
        return trends
    
    def predict_future_trends(self, historical_data: pd.DataFrame, 
                            target_column: str, days_ahead: int = 30) -> Dict[str, Any]:
        """Predict future trends using simple time series analysis"""
        try:
            # Ensure datetime index
            if not isinstance(historical_data.index, pd.DatetimeIndex):
                historical_data.index = pd.to_datetime(historical_data.index)
            
            # Simple moving average prediction
            window_size = min(30, len(historical_data) // 4)
            ma = historical_data[target_column].rolling(window=window_size).mean()
            
            # Linear trend
            x = np.arange(len(historical_data))
            y = historical_data[target_column].values
            
            # Remove NaN values
            mask = ~np.isnan(y)
            if np.sum(mask) < 2:
                return {'error': 'Insufficient data for prediction'}
            
            z = np.polyfit(x[mask], y[mask], 1)
            p = np.poly1d(z)
            
            # Project future
            future_x = np.arange(len(historical_data), len(historical_data) + days_ahead)
            future_values = p(future_x)
            
            # Add some randomness based on historical variance
            std_dev = np.std(y[mask])
            future_values += np.random.normal(0, std_dev * 0.5, days_ahead)
            
            future_dates = pd.date_range(
                start=historical_data.index[-1] + pd.Timedelta(days=1),
                periods=days_ahead,
                freq='D'
            )
            
            return {
                'dates': future_dates.tolist(),
                'predicted_values': future_values.tolist(),
                'trend': 'increasing' if z[0] > 0 else 'decreasing',
                'trend_strength': abs(z[0]),
                'confidence': 0.7  # Simple confidence metric
            }
            
        except Exception as e:
            logger.error(f"Prediction error: {e}")
            return {'error': str(e)}


class VisualizationEngine:
    """Create advanced visualizations"""
    
    def __init__(self, output_dir: Path = None):
        self.output_dir = output_dir or Path("reports/visualizations")
        self.output_dir.mkdir(parents=True, exist_ok=True)
    
    def create_repository_dashboard(self, repo_data: Dict) -> Path:
        """Create comprehensive repository dashboard"""
        fig = make_subplots(
            rows=3, cols=2,
            subplot_titles=(
                'Code Quality Metrics', 'Commit Activity',
                'Language Distribution', 'Contributor Network',
                'Issue Trends', 'Performance Metrics'
            ),
            specs=[
                [{'type': 'bar'}, {'type': 'scatter'}],
                [{'type': 'pie'}, {'type': 'scatter'}],
                [{'type': 'scatter'}, {'type': 'indicator'}]
            ]
        )
        
        # Code Quality Metrics
        metrics = repo_data.get('metrics', {})
        fig.add_trace(
            go.Bar(
                x=['Maintainability', 'Test Coverage', 'Doc Coverage', 'Security'],
                y=[
                    metrics.get('maintainability_index', 0),
                    metrics.get('test_coverage', 0),
                    metrics.get('documentation_coverage', 0),
                    metrics.get('security_score', 0)
                ],
                marker_color=['blue', 'green', 'orange', 'red']
            ),
            row=1, col=1
        )
        
        # Commit Activity
        commits = repo_data.get('commits', [])
        if commits:
            dates = [c['date'] for c in commits]
            fig.add_trace(
                go.Scatter(
                    x=dates,
                    y=list(range(len(dates))),
                    mode='lines+markers',
                    name='Cumulative Commits'
                ),
                row=1, col=2
            )
        
        # Language Distribution
        languages = repo_data.get('languages', {})
        if languages:
            fig.add_trace(
                go.Pie(
                    labels=list(languages.keys()),
                    values=list(languages.values()),
                    hole=0.4
                ),
                row=2, col=1
            )
        
        # Save dashboard
        output_path = self.output_dir / f"{repo_data.get('name', 'repo')}_dashboard.html"
        fig.write_html(str(output_path))
        
        return output_path
    
    def create_developer_insights_chart(self, dev_metrics: List[DeveloperMetrics]) -> Path:
        """Create developer productivity insights"""
        if not dev_metrics:
            return None
        
        df = pd.DataFrame([asdict(m) for m in dev_metrics])
        
        # Create scatter plot matrix
        fig = px.scatter_matrix(
            df,
            dimensions=['commits_count', 'productivity_score', 'quality_score', 'collaboration_score'],
            color='productivity_score',
            title='Developer Metrics Analysis'
        )
        
        output_path = self.output_dir / 'developer_insights.html'
        fig.write_html(str(output_path))
        
        return output_path
    
    def create_dependency_graph(self, dependencies: Dict[str, List[str]]) -> Path:
        """Create dependency visualization"""
        G = nx.Graph()
        
        # Add nodes and edges
        for repo, deps in dependencies.items():
            G.add_node(repo, node_type='repository')
            for dep in deps:
                G.add_node(dep, node_type='dependency')
                G.add_edge(repo, dep)
        
        # Calculate layout
        pos = nx.spring_layout(G, k=2, iterations=50)
        
        # Create plotly figure
        edge_trace = []
        for edge in G.edges():
            x0, y0 = pos[edge[0]]
            x1, y1 = pos[edge[1]]
            edge_trace.append(
                go.Scatter(
                    x=[x0, x1, None],
                    y=[y0, y1, None],
                    mode='lines',
                    line=dict(width=0.5, color='gray'),
                    hoverinfo='none'
                )
            )
        
        node_trace = go.Scatter(
            x=[pos[node][0] for node in G.nodes()],
            y=[pos[node][1] for node in G.nodes()],
            mode='markers+text',
            text=[node for node in G.nodes()],
            textposition='top center',
            marker=dict(
                size=10,
                color=['red' if G.nodes[node].get('node_type') == 'repository' else 'blue' 
                       for node in G.nodes()],
                line_width=2
            )
        )
        
        fig = go.Figure(
            data=edge_trace + [node_trace],
            layout=go.Layout(
                title='Dependency Graph',
                showlegend=False,
                hovermode='closest',
                margin=dict(b=0, l=0, r=0, t=40),
                xaxis=dict(showgrid=False, zeroline=False, showticklabels=False),
                yaxis=dict(showgrid=False, zeroline=False, showticklabels=False)
            )
        )
        
        output_path = self.output_dir / 'dependency_graph.html'
        fig.write_html(str(output_path))
        
        return output_path
    
    def create_heatmap(self, data: pd.DataFrame, title: str = "Activity Heatmap") -> Path:
        """Create activity heatmap"""
        fig = go.Figure(
            data=go.Heatmap(
                z=data.values,
                x=data.columns,
                y=data.index,
                colorscale='Viridis'
            )
        )
        
        fig.update_layout(
            title=title,
            xaxis_title="Time",
            yaxis_title="Activity"
        )
        
        output_path = self.output_dir / f"{title.lower().replace(' ', '_')}.html"
        fig.write_html(str(output_path))
        
        return output_path


class ReportGenerator:
    """Generate comprehensive reports"""
    
    def __init__(self, template_dir: Path = None):
        self.template_dir = template_dir or Path("templates")
        self.output_dir = Path("reports")
        self.output_dir.mkdir(parents=True, exist_ok=True)
    
    async def generate_executive_summary(self, organization_data: Dict) -> Path:
        """Generate executive summary report"""
        template = """
<!DOCTYPE html>
<html>
<head>
    <title>{{ org_name }} - Executive Summary</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .metric { display: inline-block; margin: 20px; padding: 20px; border: 1px solid #ddd; }
        .metric h3 { margin-top: 0; color: #333; }
        .metric .value { font-size: 2em; font-weight: bold; color: #0066cc; }
        .chart { margin: 20px 0; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
        .risk { color: #cc0000; }
        .opportunity { color: #00cc00; }
    </style>
</head>
<body>
    <h1>{{ org_name }} - Executive Summary</h1>
    <p>Generated: {{ generated_date }}</p>
    
    <h2>Key Metrics</h2>
    <div class="metrics">
        <div class="metric">
            <h3>Total Repositories</h3>
            <div class="value">{{ total_repos }}</div>
        </div>
        <div class="metric">
            <h3>Active Contributors</h3>
            <div class="value">{{ active_contributors }}</div>
        </div>
        <div class="metric">
            <h3>Code Health Score</h3>
            <div class="value">{{ health_score }}%</div>
        </div>
        <div class="metric">
            <h3>Security Score</h3>
            <div class="value">{{ security_score }}%</div>
        </div>
    </div>
    
    <h2>Repository Health</h2>
    <table>
        <tr>
            <th>Repository</th>
            <th>Health Score</th>
            <th>Activity</th>
            <th>Issues</th>
            <th>Tech Debt</th>
        </tr>
        {% for repo in repositories %}
        <tr>
            <td>{{ repo.name }}</td>
            <td>{{ repo.health_score }}%</td>
            <td>{{ repo.activity_level }}</td>
            <td>{{ repo.open_issues }}</td>
            <td>{{ repo.tech_debt_hours }}h</td>
        </tr>
        {% endfor %}
    </table>
    
    <h2>Risk Assessment</h2>
    <ul>
        {% for risk in risks %}
        <li class="risk">{{ risk }}</li>
        {% endfor %}
    </ul>
    
    <h2>Opportunities</h2>
    <ul>
        {% for opportunity in opportunities %}
        <li class="opportunity">{{ opportunity }}</li>
        {% endfor %}
    </ul>
    
    <h2>Recommendations</h2>
    <ol>
        {% for recommendation in recommendations %}
        <li>{{ recommendation }}</li>
        {% endfor %}
    </ol>
</body>
</html>
        """
        
        # Prepare data
        report_data = {
            'org_name': organization_data.get('name', 'Organization'),
            'generated_date': datetime.now().strftime('%Y-%m-%d %H:%M'),
            'total_repos': len(organization_data.get('repositories', [])),
            'active_contributors': organization_data.get('active_contributors', 0),
            'health_score': organization_data.get('avg_health_score', 0),
            'security_score': organization_data.get('avg_security_score', 0),
            'repositories': organization_data.get('repositories', [])[:10],  # Top 10
            'risks': organization_data.get('risks', []),
            'opportunities': organization_data.get('opportunities', []),
            'recommendations': organization_data.get('recommendations', [])
        }
        
        # Render template
        tmpl = Template(template)
        html_content = tmpl.render(**report_data)
        
        # Save report
        output_path = self.output_dir / f"executive_summary_{datetime.now().strftime('%Y%m%d')}.html"
        async with aiofiles.open(output_path, 'w') as f:
            await f.write(html_content)
        
        return output_path
    
    async def generate_technical_report(self, repo_data: Dict) -> Path:
        """Generate detailed technical report"""
        # Similar structure but with more technical details
        # Implementation would include code quality metrics, dependency analysis,
        # security vulnerabilities, performance metrics, etc.
        pass
    
    async def generate_pdf_report(self, html_path: Path) -> Path:
        """Convert HTML report to PDF"""
        # Would use wkhtmltopdf or similar
        # For now, return HTML path
        return html_path


class AnalyticsEngine:
    """Main analytics engine combining all components"""
    
    def __init__(self):
        self.metrics_calculator = MetricsCalculator()
        self.trend_analyzer = TrendAnalyzer()
        self.visualization_engine = VisualizationEngine()
        self.report_generator = ReportGenerator()
        self.cache = {}
    
    async def analyze_repository(self, repo_path: Path, repo_data: Dict) -> RepositoryInsights:
        """Perform comprehensive repository analysis"""
        # Calculate metrics
        code_metrics = await self.metrics_calculator.calculate_code_metrics(repo_path)
        
        # Analyze trends
        commit_trends = self.trend_analyzer.analyze_commit_trends(
            repo_data.get('commits', [])
        )
        issue_trends = self.trend_analyzer.analyze_issue_trends(
            repo_data.get('issues', [])
        )
        
        # Calculate health score
        health_score = self._calculate_health_score(code_metrics, commit_trends, issue_trends)
        
        # Determine activity level
        activity_level = self._determine_activity_level(commit_trends)
        
        # Identify risks
        risk_factors = self._identify_risks(code_metrics, issue_trends)
        
        # Generate recommendations
        recommendations = self._generate_recommendations(code_metrics, risk_factors)
        
        return RepositoryInsights(
            repo_name=repo_data.get('name', 'Unknown'),
            primary_language=repo_data.get('language', 'Unknown'),
            health_score=health_score,
            activity_level=activity_level,
            contributor_diversity=self._calculate_contributor_diversity(commit_trends),
            issue_resolution_time=issue_trends.get('avg_resolution_days', 0),
            pr_merge_time=repo_data.get('avg_pr_merge_time', 0),
            code_quality_trend=self._determine_quality_trend(code_metrics),
            risk_factors=risk_factors,
            recommendations=recommendations
        )
    
    def _calculate_health_score(self, metrics: CodeMetrics, 
                              commit_trends: Dict, issue_trends: Dict) -> float:
        """Calculate overall repository health score"""
        weights = {
            'maintainability': 0.25,
            'test_coverage': 0.20,
            'security': 0.20,
            'activity': 0.15,
            'issue_resolution': 0.10,
            'documentation': 0.10
        }
        
        scores = {
            'maintainability': metrics.maintainability_index,
            'test_coverage': metrics.test_coverage,
            'security': metrics.security_score,
            'activity': min(100, commit_trends.get('commits_per_monthly', 0) * 10),
            'issue_resolution': max(0, 100 - issue_trends.get('avg_resolution_days', 30)),
            'documentation': metrics.documentation_coverage
        }
        
        weighted_score = sum(scores[k] * weights[k] for k in weights)
        return round(weighted_score, 1)
    
    def _determine_activity_level(self, commit_trends: Dict) -> str:
        """Determine repository activity level"""
        monthly_commits = commit_trends.get('commits_per_monthly', 0)
        
        if monthly_commits >= 100:
            return 'Very High'
        elif monthly_commits >= 50:
            return 'High'
        elif monthly_commits >= 20:
            return 'Moderate'
        elif monthly_commits >= 5:
            return 'Low'
        else:
            return 'Inactive'
    
    def _calculate_contributor_diversity(self, commit_trends: Dict) -> float:
        """Calculate contributor diversity score"""
        contributors = commit_trends.get('top_contributors', {})
        if not contributors:
            return 0
        
        # Shannon diversity index
        total = sum(contributors.values())
        if total == 0:
            return 0
        
        proportions = [count/total for count in contributors.values()]
        diversity = -sum(p * np.log(p) for p in proportions if p > 0)
        
        # Normalize to 0-100
        max_diversity = np.log(len(contributors))
        return (diversity / max_diversity * 100) if max_diversity > 0 else 0
    
    def _determine_quality_trend(self, metrics: CodeMetrics) -> str:
        """Determine code quality trend"""
        # In a real implementation, this would compare historical metrics
        # For now, use current metrics
        if metrics.maintainability_index >= 80:
            return 'Improving'
        elif metrics.maintainability_index >= 60:
            return 'Stable'
        else:
            return 'Declining'
    
    def _identify_risks(self, metrics: CodeMetrics, issue_trends: Dict) -> List[str]:
        """Identify risk factors"""
        risks = []
        
        if metrics.maintainability_index < 50:
            risks.append('Low maintainability index indicates high technical debt')
        
        if metrics.test_coverage < 60:
            risks.append('Insufficient test coverage increases regression risk')
        
        if metrics.security_score < 70:
            risks.append('Security vulnerabilities detected')
        
        if issue_trends.get('open_rate', 0) > 0.5:
            risks.append('High ratio of open issues indicates maintenance challenges')
        
        if metrics.dependency_count > 100:
            risks.append('High number of dependencies increases supply chain risk')
        
        return risks
    
    def _generate_recommendations(self, metrics: CodeMetrics, risks: List[str]) -> List[str]:
        """Generate actionable recommendations"""
        recommendations = []
        
        if metrics.test_coverage < 80:
            recommendations.append(
                f'Increase test coverage from {metrics.test_coverage:.0f}% to at least 80%'
            )
        
        if metrics.documentation_coverage < 70:
            recommendations.append(
                'Improve documentation coverage for better maintainability'
            )
        
        if metrics.code_duplication_ratio > 10:
            recommendations.append(
                f'Reduce code duplication (currently {metrics.code_duplication_ratio:.1f}%)'
            )
        
        if 'security' in str(risks).lower():
            recommendations.append(
                'Conduct security audit and implement recommended fixes'
            )
        
        if metrics.technical_debt_hours > 100:
            recommendations.append(
                f'Allocate time to address {metrics.technical_debt_hours:.0f} hours of technical debt'
            )
        
        return recommendations
    
    async def generate_organization_report(self, org_data: Dict) -> Path:
        """Generate comprehensive organization report"""
        # Analyze all repositories
        insights = []
        for repo in org_data.get('repositories', []):
            repo_path = Path(repo.get('local_path', ''))
            if repo_path.exists():
                insight = await self.analyze_repository(repo_path, repo)
                insights.append(insight)
        
        # Calculate organization-level metrics
        org_data['avg_health_score'] = np.mean([i.health_score for i in insights])
        org_data['avg_security_score'] = np.mean([
            i.health_score for i in insights  # Would be security-specific
        ])
        
        # Identify organization-wide risks and opportunities
        all_risks = []
        for insight in insights:
            all_risks.extend(insight.risk_factors)
        
        org_data['risks'] = list(set(all_risks))[:5]  # Top 5 unique risks
        
        # Generate opportunities based on analysis
        org_data['opportunities'] = [
            'Standardize testing practices across repositories',
            'Implement automated security scanning',
            'Create shared component libraries to reduce duplication',
            'Establish code review guidelines'
        ]
        
        # Generate recommendations
        org_data['recommendations'] = [
            'Implement organization-wide code quality standards',
            'Set up continuous integration for all active repositories',
            'Create documentation templates and guidelines',
            'Establish security response team and procedures'
        ]
        
        # Generate executive summary
        report_path = await self.report_generator.generate_executive_summary(org_data)
        
        # Create visualizations
        for repo in org_data.get('repositories', [])[:5]:  # Top 5 repos
            self.visualization_engine.create_repository_dashboard(repo)
        
        return report_path