use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::fs;

/// Metrics collected during repository processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryMetrics {
    /// Name of the repository
    pub name: String,
    /// Number of files processed
    pub files_processed: usize,
    /// Total lines of code
    pub total_lines: usize,
    /// Processing duration in seconds
    pub duration_secs: f64,
    /// Memory usage in MB
    pub memory_mb: f64,
    /// Lines of code (excluding comments and blank lines)
    pub lines_of_code: usize,
    /// Number of commits in the repository
    pub commit_count: u32,
    /// Number of unique contributors
    pub contributor_count: u32,
    /// Distribution of languages in the repository
    pub language_distribution: HashMap<String, f32>,
    /// Overall complexity score (0-100)
    pub complexity_score: f32,
    /// Timestamp of when the metrics were collected
    pub timestamp: Option<String>,
}

impl RepositoryMetrics {
    /// Create new metrics instance
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            files_processed: 0,
            total_lines: 0,
            duration_secs: 0.0,
            memory_mb: 0.0,
            lines_of_code: 0,
            commit_count: 0,
            contributor_count: 0,
            language_distribution: HashMap::new(),
            complexity_score: 0.0,
            timestamp: Some(chrono::Utc::now().to_rfc3339()),
        }
    }

    /// Export metrics to JSON file
    pub fn export(&self, output_dir: &Path) -> anyhow::Result<()> {
        let metrics_file = output_dir.join("metrics.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(metrics_file, json)?;
        Ok(())
    }
    
    /// Start timing a new operation
    pub fn start_timer(&self) -> Instant {
        Instant::now()
    }
    
    /// Calculate duration from a previously started timer
    pub fn end_timer(&mut self, start: Instant) {
        self.duration_secs = start.elapsed().as_secs_f64();
    }
    
    /// Update language distribution for a given file
    pub fn update_language(&mut self, language: &str, weight: f32) {
        let entry = self.language_distribution.entry(language.to_string()).or_insert(0.0);
        *entry += weight;
    }
    
    /// Merge with another metrics object
    pub fn merge(&mut self, other: &RepositoryMetrics) {
        self.files_processed += other.files_processed;
        self.total_lines += other.total_lines;
        self.lines_of_code += other.lines_of_code;
        
        // We keep the max duration as an approximation (as operations might be parallelized)
        self.duration_secs = self.duration_secs.max(other.duration_secs);
        
        // Sum the memory usage
        self.memory_mb += other.memory_mb;
        
        // Take max of commit counts and contributor counts 
        // (assuming we're analyzing the same repo in different ways)
        self.commit_count = self.commit_count.max(other.commit_count);
        self.contributor_count = self.contributor_count.max(other.contributor_count);
        
        // Merge language distributions
        for (lang, weight) in &other.language_distribution {
            let entry = self.language_distribution.entry(lang.clone()).or_insert(0.0);
            *entry += weight;
        }
        
        // Recalculate complexity score as weighted average
        let total_loc = self.lines_of_code as f32;
        if total_loc > 0.0 {
            let self_weight = self.lines_of_code as f32 / total_loc;
            let other_weight = other.lines_of_code as f32 / total_loc;
            self.complexity_score = 
                (self.complexity_score * self_weight + other.complexity_score * other_weight) / 
                (self_weight + other_weight);
        }
    }
}

/// Analytics processor for collecting and analyzing metrics
pub struct AnalyticsProcessor {
    metrics: Option<RepositoryMetrics>,
    start_time: Option<Instant>,
}

impl AnalyticsProcessor {
    /// Create a new analytics processor
    pub fn new() -> Self {
        Self {
            metrics: None,
            start_time: None,
        }
    }
    
    /// Start tracking metrics for a repository
    pub fn start_repository(&mut self, repo_name: &str) {
        self.metrics = Some(RepositoryMetrics::new(repo_name));
        self.start_time = Some(Instant::now());
    }
    
    /// Record file processing
    pub fn record_file(&mut self, file_path: &Path, line_count: usize) {
        if let Some(metrics) = &mut self.metrics {
            metrics.files_processed += 1;
            metrics.total_lines += line_count;
            
            // Estimate actual code lines (non-blank, non-comment) as 70% of total
            // This is just a heuristic - in a real implementation you'd want to actually count
            metrics.lines_of_code += (line_count as f32 * 0.7) as usize;
            
            // Update language based on file extension
            if let Some(ext) = file_path.extension() {
                if let Some(ext_str) = ext.to_str() {
                    let language = match ext_str {
                        "rs" => "Rust",
                        "py" => "Python",
                        "js" => "JavaScript",
                        "ts" => "TypeScript",
                        "go" => "Go",
                        "c" | "cpp" | "h" | "hpp" => "C/C++",
                        "java" => "Java",
                        "kt" => "Kotlin",
                        "rb" => "Ruby",
                        "php" => "PHP",
                        "html" => "HTML",
                        "css" => "CSS",
                        "md" => "Markdown",
                        "json" => "JSON",
                        "yml" | "yaml" => "YAML",
                        "toml" => "TOML",
                        _ => "Other",
                    };
                    
                    metrics.update_language(language, line_count as f32);
                }
            }
        }
    }
    
    /// Finish processing and return the metrics
    pub fn finish(&mut self) -> Option<RepositoryMetrics> {
        if let (Some(metrics), Some(start_time)) = (&mut self.metrics, self.start_time) {
            metrics.duration_secs = start_time.elapsed().as_secs_f64();
            
            // Calculate complexity score based on various factors
            // This is a simplified heuristic
            let loc_factor = (metrics.lines_of_code as f32).log10() / 5.0; // 0.0-1.0 based on code size
            let lang_factor = metrics.language_distribution.len() as f32 / 5.0; // 0.0-1.0 based on language diversity
            
            metrics.complexity_score = (loc_factor * 0.7 + lang_factor * 0.3) * 100.0;
            metrics.complexity_score = metrics.complexity_score.min(100.0);
            
            // Normalize language distribution percentages
            let total_lines: f32 = metrics.language_distribution.values().sum();
            if total_lines > 0.0 {
                for value in metrics.language_distribution.values_mut() {
                    *value = *value / total_lines;
                }
            }
            
            return Some(metrics.clone());
        }
        None
    }
    
    /// Export metrics to a file
    pub fn export(&self, output_dir: &Path) -> anyhow::Result<()> {
        if let Some(metrics) = &self.metrics {
            metrics.export(output_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_new_metrics() {
        let metrics = RepositoryMetrics::new("test-repo");
        assert_eq!(metrics.name, "test-repo");
        assert_eq!(metrics.files_processed, 0);
        assert_eq!(metrics.total_lines, 0);
        assert!(metrics.timestamp.is_some());
    }
    
    #[test]
    fn test_export_metrics() {
        let dir = tempdir().unwrap();
        let metrics = RepositoryMetrics::new("test-repo");
        
        assert!(metrics.export(dir.path()).is_ok());
        
        let exported_file = dir.path().join("metrics.json");
        assert!(exported_file.exists());
        
        let content = fs::read_to_string(exported_file).unwrap();
        let deserialized: RepositoryMetrics = serde_json::from_str(&content).unwrap();
        assert_eq!(deserialized.name, "test-repo");
    }
    
    #[test]
    fn test_analytics_processor() {
        let mut processor = AnalyticsProcessor::new();
        
        // Start tracking a repository
        processor.start_repository("test-repo");
        
        // Record some files
        let rust_file = Path::new("src/main.rs");
        let python_file = Path::new("scripts/test.py");
        
        processor.record_file(rust_file, 100);
        processor.record_file(python_file, 50);
        
        // Get final metrics
        let metrics = processor.finish().unwrap();
        
        assert_eq!(metrics.name, "test-repo");
        assert_eq!(metrics.files_processed, 2);
        assert_eq!(metrics.total_lines, 150);
        assert!(metrics.duration_secs > 0.0);
        
        // Check language distribution
        assert!(metrics.language_distribution.contains_key("Rust"));
        assert!(metrics.language_distribution.contains_key("Python"));
        
        let rust_pct = metrics.language_distribution["Rust"];
        let python_pct = metrics.language_distribution["Python"];
        
        assert!(rust_pct > python_pct); // Rust should have a higher percentage
        
        // Check that percentages add up to approximately 1.0
        let total: f32 = metrics.language_distribution.values().sum();
        assert!((total - 1.0).abs() < 0.001);
    }
}
