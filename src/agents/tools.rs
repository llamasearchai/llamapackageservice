//! Tools and utilities for OpenAI agents
//!
//! This module provides utility functions and tools for enhancing AI agent capabilities.

use crate::error::Result;
use std::path::Path;
use serde::{Deserialize, Serialize};

/// Repository analysis tools
pub struct RepositoryTools;

impl RepositoryTools {
    /// Extract key information from a repository
    pub async fn extract_repository_summary(path: &Path) -> Result<RepositorySummary> {
        let summary = RepositorySummary {
            name: path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            primary_language: Self::detect_primary_language(path).await?,
            file_count: Self::count_files(path).await?,
            total_size: Self::calculate_size(path).await?,
            key_files: Self::identify_key_files(path).await?,
        };
        
        Ok(summary)
    }

    /// Detect the primary programming language
    async fn detect_primary_language(path: &Path) -> Result<String> {
        // Simple language detection based on file extensions
        let mut language_counts = std::collections::HashMap::new();
        
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(extension) = entry.path().extension() {
                    if let Some(ext_str) = extension.to_str() {
                        let language = match ext_str {
                            "rs" => "Rust",
                            "py" => "Python",
                            "js" | "jsx" => "JavaScript",
                            "ts" | "tsx" => "TypeScript",
                            "go" => "Go",
                            "java" => "Java",
                            "cpp" | "cc" | "cxx" => "C++",
                            "c" => "C",
                            _ => continue,
                        };
                        *language_counts.entry(language).or_insert(0) += 1;
                    }
                }
            }
        }
        
        let primary = language_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(lang, _)| lang.to_string())
            .unwrap_or_else(|| "Unknown".to_string());
            
        Ok(primary)
    }

    /// Count files in the repository
    async fn count_files(path: &Path) -> Result<usize> {
        let mut count = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            count = entries.count();
        }
        Ok(count)
    }

    /// Calculate total repository size
    async fn calculate_size(path: &Path) -> Result<u64> {
        let mut size = 0;
        if let Ok(metadata) = std::fs::metadata(path) {
            if metadata.is_file() {
                size = metadata.len();
            } else if metadata.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten() {
                        if let Ok(entry_metadata) = entry.metadata() {
                            size += entry_metadata.len();
                        }
                    }
                }
            }
        }
        Ok(size)
    }

    /// Identify key files in the repository
    async fn identify_key_files(path: &Path) -> Result<Vec<String>> {
        let mut key_files = Vec::new();
        let important_files = [
            "README.md", "README.txt", "README",
            "Cargo.toml", "package.json", "setup.py",
            "requirements.txt", "go.mod", "pom.xml",
            "LICENSE", "LICENSE.txt", "LICENSE.md",
        ];

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                if let Some(filename) = entry.file_name().to_str() {
                    if important_files.contains(&filename) {
                        key_files.push(filename.to_string());
                    }
                }
            }
        }

        Ok(key_files)
    }
}

/// Summary of a repository's key characteristics
#[derive(Debug, Serialize, Deserialize)]
pub struct RepositorySummary {
    /// Repository name
    pub name: String,
    /// Primary programming language
    pub primary_language: String,
    /// Number of files
    pub file_count: usize,
    /// Total size in bytes
    pub total_size: u64,
    /// Key files found
    pub key_files: Vec<String>,
}

/// Code analysis utilities
pub struct CodeAnalysisTools;

impl CodeAnalysisTools {
    /// Calculate basic complexity metrics
    pub fn calculate_complexity(code: &str) -> ComplexityMetrics {
        let lines = code.lines().count();
        let non_empty_lines = code.lines().filter(|line| !line.trim().is_empty()).count();
        let comment_lines = code.lines().filter(|line| line.trim().starts_with("//") || line.trim().starts_with('#')).count();
        
        ComplexityMetrics {
            total_lines: lines,
            code_lines: non_empty_lines - comment_lines,
            comment_lines,
            cyclomatic_complexity: Self::estimate_cyclomatic_complexity(code),
        }
    }

    /// Estimate cyclomatic complexity
    fn estimate_cyclomatic_complexity(code: &str) -> u32 {
        let complexity_keywords = ["if", "else", "while", "for", "match", "case", "catch", "?"];
        let mut complexity = 1; // Base complexity
        
        for line in code.lines() {
            for keyword in &complexity_keywords {
                complexity += line.matches(keyword).count() as u32;
            }
        }
        
        complexity
    }
}

/// Code complexity metrics
#[derive(Debug, Serialize, Deserialize)]
pub struct ComplexityMetrics {
    /// Total lines of code
    pub total_lines: usize,
    /// Lines of actual code (excluding comments and empty lines)
    pub code_lines: usize,
    /// Lines of comments
    pub comment_lines: usize,
    /// Estimated cyclomatic complexity
    pub cyclomatic_complexity: u32,
} 