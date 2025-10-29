use crate::error::{ProcessorError, Result};
use crate::config::Config;
use crate::processors::common::{self, save_output_file, setup_progress_style, create_progress_bar};
use crate::processors::PackageProcessor;
use std::fs as std_fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::io;
use async_trait::async_trait;
use chrono::Utc;
use indicatif::ProgressBar;
use log::{info, warn};
use serde::{Serialize, Deserialize};
use tokio::fs as tokio_fs;
use walkdir::WalkDir;
use std::pin::Pin;
use std::boxed::Box;
use futures::future::BoxFuture;

const LOCAL_REPOS_DIR: &str = "local_repositories";
const ANALYSIS_SUFFIX: &str = "analysis";

// Supported file extensions for code analysis
const CODE_EXTENSIONS: &[&str] = &[
    ".rs", ".go", ".c", ".cpp", ".h", ".hpp", ".cc", ".cxx",
    ".js", ".ts", ".jsx", ".tsx", ".html", ".css", ".scss", ".sass", ".less",
    ".py", ".rb", ".php", ".sh", ".bash", ".zsh", ".fish", ".ps1",
    ".java", ".kt", ".scala", ".groovy", ".clj", ".cljs",
    ".cs", ".fs", ".vb",
    ".swift", ".m", ".mm", ".dart",
    ".hs", ".elm", ".ml", ".ocaml", ".f", ".for", ".f90",
    ".json", ".yml", ".yaml", ".toml", ".xml", ".ini", ".cfg", ".conf",
    ".md", ".rst", ".txt", ".tex", ".org",
    ".makefile", ".cmake", ".gradle", ".build", ".bazel",
    ".sql", ".sqlite", ".db",
    ".r", ".lua", ".vim", ".emacs", ".dockerfile"
];

// Files to ignore during processing
const IGNORE_PATTERNS: &[&str] = &[
    ".git", ".svn", ".hg", ".bzr",
    "node_modules", "target", "build", "dist", "out",
    ".vscode", ".idea", ".DS_Store", "Thumbs.db",
    "*.log", "*.tmp", "*.temp", "*.cache",
    "venv", ".venv", "__pycache__",
    ".pytest_cache", ".coverage", ".tox",
    "vendor", "deps", "libs"
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRepoInfo {
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub file_count: usize,
    pub primary_language: Option<String>,
    pub languages: HashMap<String, usize>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Processor for local directories and files
pub struct LocalProcessor;

impl LocalProcessor {
    /// Create a new local processor
    pub fn new() -> Self {
        Self
    }

    /// Check if a path is a local file or directory
    pub fn is_local_path(path: &str) -> bool {
        let path = path.trim();
        // Check if it's an absolute path
        if path.starts_with('/') || path.starts_with('\\') {
            return true;
        }
        
        // Check if it's a relative path
        if path.starts_with("./") || path.starts_with("../") {
            return true;
        }
        
        // Check if it's a Windows path
        if path.len() >= 3 && path.chars().nth(1) == Some(':') {
            return true;
        }
        
        // Check if it's a simple relative path that exists
        if Path::new(path).exists() {
            return true;
        }
        
        false
    }

    /// Process a local directory or file
    async fn process_path(&self, path: &str, output_dir: &Path, _config: &Config) -> Result<()> {
        let normalized = crate::utils::normalize_user_input_path(path);
        let input_path = normalized.as_path();
        
        // Check existence first
        if !input_path.exists() {
            return Err(ProcessorError::Message(format!(
                "Path does not exist: '{}'. Please verify the path is correct.",
                input_path.display()
            )));
        }
        
        // Check read permissions - but continue anyway
        match std_fs::metadata(input_path) {
            Ok(_metadata) => {
                // Permissions checked successfully, proceed
            }
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                // Log warning but continue processing - we'll skip inaccessible parts
                warn!("Permission denied for root path: '{}'. Will attempt to process accessible parts.", input_path.display());
            }
            Err(_e) => {
                // Other metadata errors - log and continue
                warn!("Could not read metadata for: '{}'. Will attempt to process.", input_path.display());
            }
        }

        let pb = create_progress_bar();
        setup_progress_style(&pb);
        
        if input_path.is_file() {
            pb.set_message("Processing file...");
            // Try to process the file, skip if permission denied
            match self.process_single_file(input_path, output_dir, &pb).await {
                Ok(_) => {},
                Err(e) => {
                    warn!("Could not process file: {}", e);
                    // Continue anyway - don't fail
                }
            }
        } else if input_path.is_dir() {
            pb.set_message("Processing directory...");
            // Process directory, skipping inaccessible parts
            match self.process_directory(input_path, output_dir, &pb).await {
                Ok(_) => {},
                Err(e) => {
                    warn!("Partial processing due to: {}", e);
                    // Continue anyway - we processed what we could
                }
            }
        } else {
            warn!("Unusual path type for: '{}'. Attempting to process as directory.", input_path.display());
            // Try to process as directory anyway
            let _ = self.process_directory(input_path, output_dir, &pb).await;
        }

        pb.finish_with_message("Processing completed");
        Ok(())
    }

    /// Process a single file
    async fn process_single_file(&self, file_path: &Path, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_file");
        
        pb.set_message(format!("Analyzing file: {}", file_name));
        
        let content = tokio_fs::read_to_string(file_path).await?;
        let file_type = self.detect_file_type(file_path);
        
        let analysis = self.analyze_single_file(file_path, &content, &file_type).await?;
        
        // Create output directory for local files
        let local_output_dir = output_dir.join(LOCAL_REPOS_DIR);
        tokio_fs::create_dir_all(&local_output_dir).await?;
        
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let output_filename = format!("{}_{}_{}.txt", timestamp, file_name, ANALYSIS_SUFFIX);
        let output_path = local_output_dir.join(output_filename);
        
        save_output_file(&analysis, &output_path).await?;
        
        println!("Processed file: {} -> {}", file_path.display(), output_path.display());
        Ok(())
    }

    /// Process a directory
    async fn process_directory(&self, dir_path: &Path, output_dir: &Path, pb: &ProgressBar) -> Result<()> {
        let dir_name = dir_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown_directory");
        
        pb.set_message(format!("Analyzing directory: {}", dir_name));
        
        // Set up output path early
        let local_output_dir = output_dir.join(LOCAL_REPOS_DIR);
        std_fs::create_dir_all(&local_output_dir)?;
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        let output_filename = format!("{}_{}_{}.txt", timestamp, dir_name, ANALYSIS_SUFFIX);
        let output_path = local_output_dir.join(output_filename);
        
        // Collect all files first
        let files = self.collect_files(dir_path)?;
        let file_count = files.len();
        
        if file_count == 0 {
            warn!("No accessible files found in directory: {}", dir_path.display());
            // Create a minimal analysis file anyway
            let mut analysis = String::new();
            analysis.push_str(&format!("# Local Repository Analysis: {}\n\n", dir_name));
            analysis.push_str(&format!("**Author:** Nik Jois <nikjois@llamasearch.ai>\n"));
            analysis.push_str(&format!("**Generated:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
            analysis.push_str("## Status\n\n");
            analysis.push_str("No accessible files found.\n");
            analysis.push_str("All files may be inaccessible due to permissions or the directory may be empty.\n");
            
            save_output_file(&analysis, &output_path).await?;
            info!("Created minimal analysis at: {}", output_path.display());
            return Ok(());
        }
        
        pb.set_length(file_count as u64);
        
        // Analyze the repository structure
        let repo_info = self.analyze_repository(dir_path, &files).await?;
        
        // Generate comprehensive analysis
        let mut analysis = String::new();
        analysis.push_str(&format!("# Local Repository Analysis: {}\n\n", dir_name));
        analysis.push_str(&format!("**Author:** Nik Jois <nikjois@llamasearch.ai>\n"));
        analysis.push_str(&format!("**Generated:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        // Repository overview
        analysis.push_str("## Repository Overview\n\n");
        analysis.push_str(&format!("- **Name:** {}\n", repo_info.name));
        analysis.push_str(&format!("- **Path:** {}\n", repo_info.path.display()));
        analysis.push_str(&format!("- **Size:** {} bytes\n", repo_info.size_bytes));
        analysis.push_str(&format!("- **Files:** {}\n", repo_info.file_count));
        if let Some(primary_lang) = &repo_info.primary_language {
            analysis.push_str(&format!("- **Primary Language:** {}\n", primary_lang));
        }
        analysis.push_str("\n");
        
        // Language distribution
        if !repo_info.languages.is_empty() {
            analysis.push_str("## Language Distribution\n\n");
            for (lang, count) in &repo_info.languages {
                analysis.push_str(&format!("- **{}:** {} files\n", lang, count));
            }
            analysis.push_str("\n");
        }
        
        // Directory structure
        analysis.push_str("## Directory Structure\n\n");
        analysis.push_str("```\n");
        analysis.push_str(&self.generate_directory_tree(dir_path, 0, 3).await?);
        analysis.push_str("```\n\n");

        // ----------------------------------------------------------------
        // Full source files dump
        // ----------------------------------------------------------------
        analysis.push_str("## Full Source Files\n\n");

        for file_path in files {
            let rel_path = file_path.strip_prefix(dir_path).unwrap_or(&file_path);
            pb.inc(1);
            pb.set_message(format!("Reading {}", rel_path.display()));

            analysis.push_str(&format!("### `{}`\n\n", rel_path.display()));

            let lang_hint = self.detect_file_type(&file_path);
            analysis.push_str(&format!("```{}\n", lang_hint));

            if let Ok(src) = tokio_fs::read_to_string(file_path).await {
                analysis.push_str(&src);
            } else {
                analysis.push_str("/* Unable to read file */");
            }

            analysis.push_str("\n```\n\n");
        }
        
        // Save the analysis (output_path already defined at start of function)
        save_output_file(&analysis, &output_path).await?;
        
        println!("Processed directory: {} -> {}", dir_path.display(), output_path.display());
        Ok(())
    }

    /// Collect all files in a directory, respecting ignore patterns
    /// Always continues processing, skipping any inaccessible files/directories
    fn collect_files(&self, dir_path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        for entry in WalkDir::new(dir_path)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !self.should_ignore(e.path()))
        {
            match entry {
                Ok(entry) => {
                    if entry.file_type().is_file() {
                        files.push(entry.path().to_path_buf());
                    }
                }
                Err(e) => {
                    // Log any error but always continue processing
                    let path_str = e.path()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| "unknown path".to_string());
                    
                    if let Some(io_err) = e.io_error() {
                        match io_err.kind() {
                            std::io::ErrorKind::PermissionDenied => {
                                warn!("Skipping (permission denied): {}", path_str);
                            }
                            _ => {
                                warn!("Skipping (error): {} - {}", path_str, io_err);
                            }
                        }
                    } else {
                        warn!("Skipping (unknown error): {}", path_str);
                    }
                    
                    // Always continue - never fail
                    continue;
                }
            }
        }
        
        // Always return success, even if we found no files
        Ok(files)
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let file_name_lower = path
            .file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        
        for pattern in IGNORE_PATTERNS {
            if pattern.contains('*') {
                // Handle common glob like "*.ext" by suffix match on file name only
                if let Some(suffix) = pattern.strip_prefix("*.") {
                    if file_name_lower.ends_with(suffix) {
                        return true;
                    }
                } else {
                    // Fallback: remove '*' and do a conservative file name contains match
                    let needle = pattern.replace('*', "").to_lowercase();
                    if !needle.is_empty() && file_name_lower.contains(&needle) {
                        return true;
                    }
                }
            } else if path_str.contains(pattern) {
                return true;
            }
        }
        
        false
    }

    /// Analyze repository structure and metadata
    async fn analyze_repository(&self, dir_path: &Path, files: &[PathBuf]) -> Result<LocalRepoInfo> {
        let name = dir_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Calculate total size
        let mut total_size = 0u64;
        for file_path in files {
            if let Ok(metadata) = tokio_fs::metadata(file_path).await {
                total_size += metadata.len();
            }
        }
        
        // Analyze languages
        let mut languages = HashMap::new();
        for file_path in files {
            if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                let lang = self.extension_to_language(ext);
                *languages.entry(lang).or_insert(0) += 1;
            }
        }
        
        // Find primary language
        let primary_language = languages.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.clone());
        
        Ok(LocalRepoInfo {
            name,
            path: dir_path.to_path_buf(),
            size_bytes: total_size,
            file_count: files.len(),
            primary_language,
            languages,
            created_at: Utc::now(),
        })
    }

    /// Detect file type based on extension and content
    fn detect_file_type(&self, file_path: &Path) -> String {
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            match ext.to_lowercase().as_str() {
                "md" | "rst" | "txt" | "doc" | "docx" | "pdf" => "Documentation".to_string(),
                "json" | "yml" | "yaml" | "toml" | "xml" | "ini" | "cfg" | "conf" => "Configuration".to_string(),
                ext if CODE_EXTENSIONS.contains(&format!(".{}", ext).as_str()) => "Code".to_string(),
                _ => "Other".to_string(),
            }
        } else {
            "Other".to_string()
        }
    }

    /// Convert file extension to language name
    fn extension_to_language(&self, ext: &str) -> String {
        match ext.to_lowercase().as_str() {
            "rs" => "Rust".to_string(),
            "py" => "Python".to_string(),
            "js" => "JavaScript".to_string(),
            "ts" => "TypeScript".to_string(),
            "java" => "Java".to_string(),
            "cpp" | "cc" | "cxx" => "C++".to_string(),
            "c" => "C".to_string(),
            "h" | "hpp" => "C/C++ Header".to_string(),
            "go" => "Go".to_string(),
            "rb" => "Ruby".to_string(),
            "php" => "PHP".to_string(),
            "swift" => "Swift".to_string(),
            "kt" => "Kotlin".to_string(),
            "scala" => "Scala".to_string(),
            "cs" => "C#".to_string(),
            "fs" => "F#".to_string(),
            "hs" => "Haskell".to_string(),
            "ml" => "OCaml".to_string(),
            "r" => "R".to_string(),
            "dart" => "Dart".to_string(),
            "lua" => "Lua".to_string(),
            "sh" | "bash" | "zsh" => "Shell".to_string(),
            "html" => "HTML".to_string(),
            "css" => "CSS".to_string(),
            "scss" | "sass" => "SCSS/SASS".to_string(),
            "json" => "JSON".to_string(),
            "yml" | "yaml" => "YAML".to_string(),
            "toml" => "TOML".to_string(),
            "xml" => "XML".to_string(),
            "md" => "Markdown".to_string(),
            "sql" => "SQL".to_string(),
            _ => "Other".to_string(),
        }
    }

    /// Generate directory tree representation
    fn generate_directory_tree<'a>(&'a self, dir_path: &'a Path, current_depth: usize, max_depth: usize) -> BoxFuture<'a, Result<String>> {
        Box::pin(async move {
            if current_depth >= max_depth {
                return Ok("...\n".to_string());
            }
            
            let mut tree = String::new();
            let indent = "  ".repeat(current_depth);
            
            let mut entries = Vec::new();
            let mut read_dir = tokio_fs::read_dir(dir_path).await?;
            
            while let Some(entry) = read_dir.next_entry().await? {
                let path = entry.path();
                if !self.should_ignore(&path) {
                    entries.push(path);
                }
            }
            
            // Sort entries: directories first, then files
            entries.sort_by(|a, b| {
                match (a.is_dir(), b.is_dir()) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.file_name().cmp(&b.file_name()),
                }
            });
            
            for entry in entries.iter().take(50) {
                let name = entry.file_name().unwrap_or_default().to_string_lossy();
                
                if entry.is_dir() {
                    tree.push_str(&format!("{}[DIR] {}/\n", indent, name));
                    if current_depth < max_depth - 1 {
                        tree.push_str(&self.generate_directory_tree(entry, current_depth + 1, max_depth).await?);
                    }
                } else {
                    tree.push_str(&format!("{}[FILE] {}\n", indent, name));
                }
            }
            
            Ok(tree)
        })
    }

    /// Analyze a single file
    async fn analyze_single_file(&self, file_path: &Path, content: &str, file_type: &str) -> Result<String> {
        let mut analysis = String::new();
        let file_name = file_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        
        analysis.push_str(&format!("# File Analysis: {}\n\n", file_name));
        analysis.push_str(&format!("**Author:** Nik Jois <nikjois@llamasearch.ai>\n"));
        analysis.push_str(&format!("**Generated:** {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        analysis.push_str("## File Information\n\n");
        analysis.push_str(&format!("- **Path:** {}\n", file_path.display()));
        analysis.push_str(&format!("- **Type:** {}\n", file_type));
        analysis.push_str(&format!("- **Size:** {} bytes\n", content.len()));
        analysis.push_str(&format!("- **Lines:** {}\n", content.lines().count()));
        
        if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
            analysis.push_str(&format!("- **Language:** {}\n", self.extension_to_language(ext)));
        }
        
        analysis.push_str("\n## File Content\n\n");
        analysis.push_str("```\n");
        analysis.push_str(content);
        analysis.push_str("\n```\n");
        
        Ok(analysis)
    }
}

#[async_trait]
impl PackageProcessor for LocalProcessor {
    fn name(&self) -> &'static str {
        "Local File/Directory Processor"
    }

    fn accepts(&self, url: &str) -> bool {
        Self::is_local_path(url)
    }

    async fn validate(&self, url: &str) -> Result<()> {
        let normalized = crate::utils::normalize_user_input_path(url);
        let path = normalized.as_path();
        if !path.exists() {
            return Err(ProcessorError::Message(format!("Path does not exist: {}", path.display())));
        }
        Ok(())
    }

    async fn process(&self, url: &str, output_dir: &Path, config: &Config) -> Result<()> {
        self.process_path(url, output_dir, config).await
    }
}

impl Clone for LocalProcessor {
    fn clone(&self) -> Self {
        Self::new()
    }
}
