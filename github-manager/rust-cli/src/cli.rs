use clap::{Parser, Subcommand, Args};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "llamasearch",
    about = "LlamaSearchAI GitHub Repository Manager",
    version,
    author,
    long_about = "Advanced GitHub organization management with terminal browser"
)]
pub struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "~/.llamasearch/config.toml")]
    pub config: PathBuf,
    
    /// Verbosity level
    #[arg(short, long, action = clap::ArgAction::Count)]
    pub verbose: u8,
    
    /// Subcommand to execute
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Launch terminal UI browser
    Tui,
    
    /// Scan repositories
    Scan(ScanArgs),
    
    /// Clone repositories
    Clone(CloneArgs),
    
    /// Generate concatenated files
    Generate(GenerateArgs),
    
    /// Analyze repositories
    Analyze(AnalyzeArgs),
    
    /// Search across repositories
    Search(SearchArgs),
    
    /// Manage workflows
    Workflow(WorkflowArgs),
    
    /// Security operations
    Security(SecurityArgs),
    
    /// Performance analysis
    Performance(PerformanceArgs),
    
    /// Repository operations
    Repo(RepoArgs),
    
    /// Configuration management
    Config(ConfigArgs),
    
    /// Authentication
    Auth(AuthArgs),
}

#[derive(Args, Debug)]
pub struct ScanArgs {
    /// Organization name
    #[arg(short, long, default_value = "llamasearchai")]
    pub org: String,
    
    /// Include archived repositories
    #[arg(long)]
    pub include_archived: bool,
    
    /// Output format
    #[arg(short, long, value_enum, default_value = "table")]
    pub format: OutputFormat,
    
    /// Filter by language
    #[arg(short, long)]
    pub language: Option<String>,
    
    /// Sort by field
    #[arg(long, value_enum, default_value = "stars")]
    pub sort: SortField,
}

#[derive(Args, Debug)]
pub struct CloneArgs {
    /// Repository names to clone (or "all")
    pub repos: Vec<String>,
    
    /// Target directory
    #[arg(short, long)]
    pub target: Option<PathBuf>,
    
    /// Clone depth
    #[arg(long, default_value = "1")]
    pub depth: u32,
    
    /// Update existing repos
    #[arg(short, long)]
    pub update: bool,
    
    /// Parallel clone operations
    #[arg(short, long, default_value = "4")]
    pub parallel: usize,
}

#[derive(Args, Debug)]
pub struct GenerateArgs {
    /// Repository names (or "all")
    pub repos: Vec<String>,
    
    /// Output directory
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    
    /// Output formats
    #[arg(short, long, value_delimiter = ',')]
    pub formats: Vec<FileFormat>,
    
    /// Include git history
    #[arg(long)]
    pub include_history: bool,
    
    /// Compression
    #[arg(short, long)]
    pub compress: bool,
    
    /// Maximum file size in MB
    #[arg(long, default_value = "100")]
    pub max_size: u64,
}

#[derive(Args, Debug)]
pub struct AnalyzeArgs {
    /// Repository name
    pub repo: String,
    
    /// Analysis type
    #[arg(short, long, value_enum)]
    pub analysis_type: Vec<AnalysisType>,
    
    /// Output report
    #[arg(short, long)]
    pub report: bool,
    
    /// Interactive mode
    #[arg(short, long)]
    pub interactive: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query
    pub query: String,
    
    /// Repository scope
    #[arg(short, long)]
    pub repo: Option<String>,
    
    /// File pattern
    #[arg(short, long)]
    pub pattern: Option<String>,
    
    /// Case sensitive
    #[arg(short, long)]
    pub case_sensitive: bool,
    
    /// Regular expression
    #[arg(short, long)]
    pub regex: bool,
    
    /// Context lines
    #[arg(short = 'C', long, default_value = "2")]
    pub context: usize,
}

#[derive(Args, Debug)]
pub struct WorkflowArgs {
    #[command(subcommand)]
    pub command: WorkflowCommands,
}

#[derive(Subcommand, Debug)]
pub enum WorkflowCommands {
    /// List workflows
    List,
    
    /// Run workflow
    Run {
        /// Workflow name
        name: String,
        
        /// Parameters
        #[arg(short, long)]
        params: Vec<String>,
    },
    
    /// Show workflow status
    Status {
        /// Execution ID
        id: String,
    },
    
    /// Create workflow
    Create {
        /// Workflow file
        file: PathBuf,
    },
}

#[derive(Args, Debug)]
pub struct SecurityArgs {
    #[command(subcommand)]
    pub command: SecurityCommands,
}

#[derive(Subcommand, Debug)]
pub enum SecurityCommands {
    /// Scan for vulnerabilities
    Scan {
        /// Repository name
        repo: String,
        
        /// Deep scan
        #[arg(long)]
        deep: bool,
    },
    
    /// Auto-fix vulnerabilities
    Fix {
        /// Repository name
        repo: String,
        
        /// Dry run
        #[arg(long)]
        dry_run: bool,
    },
    
    /// Audit access
    Audit {
        /// Days to audit
        #[arg(default_value = "7")]
        days: u32,
    },
}

#[derive(Args, Debug)]
pub struct PerformanceArgs {
    /// Repository name
    pub repo: String,
    
    /// Profile type
    #[arg(short, long, value_enum)]
    pub profile: ProfileType,
    
    /// Benchmark
    #[arg(short, long)]
    pub benchmark: bool,
}

#[derive(Args, Debug)]
pub struct RepoArgs {
    #[command(subcommand)]
    pub command: RepoCommands,
}

#[derive(Subcommand, Debug)]
pub enum RepoCommands {
    /// Create repository
    Create {
        /// Repository name
        name: String,
        
        /// Description
        #[arg(short, long)]
        description: Option<String>,
        
        /// Private repository
        #[arg(short, long)]
        private: bool,
    },
    
    /// Update repository
    Update {
        /// Repository name
        name: String,
        
        /// Update fields
        #[arg(short, long)]
        fields: Vec<String>,
    },
    
    /// Delete repository
    Delete {
        /// Repository name
        name: String,
        
        /// Force delete
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Show configuration
    Show,
    
    /// Set configuration value
    Set {
        /// Key
        key: String,
        
        /// Value
        value: String,
    },
    
    /// Get configuration value
    Get {
        /// Key
        key: String,
    },
    
    /// Validate configuration
    Validate,
}

#[derive(Args, Debug)]
pub struct AuthArgs {
    #[command(subcommand)]
    pub command: AuthCommands,
}

#[derive(Subcommand, Debug)]
pub enum AuthCommands {
    /// Login
    Login {
        /// Username
        #[arg(short, long)]
        username: Option<String>,
    },
    
    /// Logout
    Logout,
    
    /// Show current user
    Whoami,
    
    /// Manage tokens
    Token {
        #[command(subcommand)]
        command: TokenCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum TokenCommands {
    /// Create token
    Create {
        /// Token name
        name: String,
        
        /// Permissions
        #[arg(short, long)]
        permissions: Vec<String>,
    },
    
    /// List tokens
    List,
    
    /// Revoke token
    Revoke {
        /// Token ID
        id: String,
    },
}

// Enums for argument values

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Table,
    Json,
    Yaml,
    Csv,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum SortField {
    Name,
    Stars,
    Updated,
    Size,
    Language,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum FileFormat {
    Text,
    Markdown,
    Json,
    Html,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum AnalysisType {
    Security,
    Performance,
    Quality,
    Dependencies,
    All,
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum ProfileType {
    Cpu,
    Memory,
    Io,
    All,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;
    
    #[test]
    fn verify_cli() {
        Cli::command().debug_assert();
    }
    
    #[test]
    fn test_scan_args() {
        let args = vec!["llamasearch", "scan", "--org", "test", "--format", "json"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command {
            Some(Commands::Scan(args)) => {
                assert_eq!(args.org, "test");
                assert!(matches!(args.format, OutputFormat::Json));
            }
            _ => panic!("Expected Scan command"),
        }
    }
    
    #[test]
    fn test_search_args() {
        let args = vec!["llamasearch", "search", "TODO", "--regex", "-C", "5"];
        let cli = Cli::try_parse_from(args).unwrap();
        
        match cli.command {
            Some(Commands::Search(args)) => {
                assert_eq!(args.query, "TODO");
                assert!(args.regex);
                assert_eq!(args.context, 5);
            }
            _ => panic!("Expected Search command"),
        }
    }
}