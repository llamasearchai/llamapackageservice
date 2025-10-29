use colored::*;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use chrono::{DateTime, Utc, NaiveDateTime};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structure that manages paths to output directories for different package types
///
/// This struct provides organized access to the various output directories
/// used by the application to store processed package information.
#[derive(Debug, Clone)]
pub struct OutputPaths {
    /// Root directory for all output
    pub base_dir: PathBuf,
    /// Directory for GitHub repository outputs
    pub github_repos_dir: PathBuf,
    /// Directory for GitHub organization outputs
    pub github_orgs_dir: PathBuf,
    /// Directory for PyPI package outputs
    pub pypi_packages_dir: PathBuf,
    /// Directory for PyPI profile outputs
    pub pypi_profiles_dir: PathBuf,
    /// Directory for Rust crates outputs
    pub crates_dir: PathBuf,
    /// Directory for NPM package outputs
    pub npm_packages_dir: PathBuf,
    /// Directory for Go package outputs
    pub go_packages_dir: PathBuf,
    /// Directory for local repository outputs
    pub local_repos_dir: PathBuf,
    /// Directory for index files
    pub index_dir: PathBuf,
    /// Directory for downloaded package files
    pub downloads_dir: PathBuf,
}

impl OutputPaths {
    /// Creates a new OutputPaths structure with subdirectories based on the given base directory
    ///
    /// # Arguments
    /// * `base_dir` - The base directory to create subdirectories in
    ///
    /// # Returns
    /// A new OutputPaths instance with all subdirectories configured
    pub fn new(base_dir: &Path) -> Self {
        Self {
            base_dir: base_dir.to_path_buf(),
            github_repos_dir: base_dir.join("github_repos"),
            github_orgs_dir: base_dir.join("github_orgs"),
            pypi_packages_dir: base_dir.join("pypi_packages"),
            pypi_profiles_dir: base_dir.join("pypi_profiles"),
            crates_dir: base_dir.join("rust_crates"),
            npm_packages_dir: base_dir.join("npm_packages"),
            go_packages_dir: base_dir.join("go_packages"),
            local_repos_dir: base_dir.join("local_repositories"),
            index_dir: base_dir.join("_index"),
            downloads_dir: base_dir.join("_downloads"),
        }
    }

    /// Ensures all output directories exist, creating them if necessary
    ///
    /// # Returns
    /// IO Result indicating success or failure of directory creation
    pub fn ensure_dirs_exist(&self) -> std::io::Result<()> {
        for dir in [
            &self.base_dir,
            &self.github_repos_dir,
            &self.github_orgs_dir,
            &self.pypi_packages_dir,
            &self.pypi_profiles_dir,
            &self.crates_dir,
            &self.npm_packages_dir,
            &self.go_packages_dir,
            &self.local_repos_dir,
            &self.index_dir,
            &self.downloads_dir,
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
    
    /// Gets the appropriate directory for a given package type
    ///
    /// # Arguments
    /// * `package_type` - The type of package (e.g., "github", "pypi", "npm")
    ///
    /// # Returns
    /// Reference to the PathBuf for the corresponding package type
    pub fn get_dir_for_type(&self, package_type: &str) -> &PathBuf {
        match package_type.to_lowercase().as_str() {
            "github_repo" => &self.github_repos_dir,
            "github_org" => &self.github_orgs_dir,
            "pypi" => &self.pypi_packages_dir,
            "pypi_profile" => &self.pypi_profiles_dir,
            "crate" | "rust" => &self.crates_dir,
            "npm" => &self.npm_packages_dir,
            "go" => &self.go_packages_dir,
            "local" | "local_repo" => &self.local_repos_dir,
            _ => &self.base_dir,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PackageInfo {
    name: String,
    version: Option<String>,
    timestamp: DateTime<Utc>,
    source: String,
    description: Option<String>,
    file_path: PathBuf,
    file_size: u64,
}

/// Displays a formatted list of all output files organized by category
///
/// This function scans the output directory structure and prints a hierarchical
/// listing of all generated files, including their size and modification date.
///
/// # Arguments
/// * `output_dir` - The base output directory to list files from
pub fn list_output_files(output_dir: &Path) {
    println!("\n{}", "📂 Output Files:".bright_cyan().bold());

    let categories = [
        ("Local Repositories", "local_repositories"),
        ("GitHub Repositories", "github_repos"),
        ("GitHub Organizations", "github_orgs"),
        ("PyPI Packages", "pypi_packages"),
        ("PyPI Profiles", "pypi_profiles"),
        ("Go Packages", "go_packages"),
        ("Rust Crates", "crates"),
        ("NPM Packages", "npm_packages"),
    ];

    for (label, dir_name) in categories {
        let dir_path = output_dir.join(dir_name);
        println!("\n{}", label.bright_yellow());
        
        if dir_path.exists() {
            let mut files = Vec::new();
            for entry in WalkDir::new(&dir_path)
                .min_depth(1)
                .max_depth(1)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                files.push(entry.path().to_path_buf());
            }
            
            // Sort files by modification time (newest first)
            files.sort_by(|a, b| {
                let a_meta = fs::metadata(a).ok();
                let b_meta = fs::metadata(b).ok();
                
                match (a_meta, b_meta) {
                    (Some(a_meta), Some(b_meta)) => {
                        let a_time = a_meta.modified().unwrap_or_else(|_| std::time::SystemTime::now());
                        let b_time = b_meta.modified().unwrap_or_else(|_| std::time::SystemTime::now());
                        b_time.cmp(&a_time) // Reverse order for newest first
                    },
                    _ => std::cmp::Ordering::Equal,
                }
            });
            
            if files.is_empty() {
                println!("  └─ {}", "(empty)".dimmed());
            } else {
                for (i, path) in files.iter().enumerate() {
                    let is_last = i == files.len() - 1;
                    let prefix = if is_last { "  └─ " } else { "  ├─ " };
                    
                    let filename = path.file_name().unwrap_or_default().to_string_lossy();
                    let package_name = extract_package_name(&filename);
                    
                    // Get file size
                    if let Ok(metadata) = path.metadata() {
                        let size = metadata.len();
                        let size_str = format_file_size(size);
                        
                        // Get modified time
                        if let Ok(time) = metadata.modified() {
                            if let Ok(time) = time.duration_since(std::time::UNIX_EPOCH) {
                                let dt = DateTime::<Utc>::from_naive_utc_and_offset(
                                    DateTime::from_timestamp(time.as_secs() as i64, 0).unwrap_or_default().naive_utc(),
                                    Utc,
                                );
                                
                                let time_str = dt.format("%Y-%m-%d %H:%M").to_string();
                                println!("{}{} - {} ({}, {})", 
                                    prefix, 
                                    package_name.bright_white().bold(), 
                                    filename.bright_white(),
                                    size_str.bright_black(),
                                    time_str.bright_black());
                            } else {
                                println!("{}{} - {} ({})", 
                                    prefix, 
                                    package_name.bright_white().bold(), 
                                    filename.bright_white(),
                                    size_str.bright_black());
                            }
                        } else {
                            println!("{}{} - {} ({})", 
                                prefix, 
                                package_name.bright_white().bold(), 
                                filename.bright_white(),
                                size_str.bright_black());
                        }
                    } else {
                        println!("{}{} - {}", 
                            prefix, 
                            package_name.bright_white().bold(), 
                            filename.bright_white());
                    }
                }
            }
        } else {
            println!("  └─ {}", "(directory not found)".red());
        }
    }
}

/// Formats a file size in bytes to a human-readable string (KB, MB, etc.)
///
/// # Arguments
/// * `size` - The file size in bytes
///
/// # Returns
/// A formatted string representation of the file size with appropriate units
pub fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{}B", size)
    } else if size < 1024 * 1024 {
        format!("{:.1}KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1}MB", size as f64 / 1024.0 / 1024.0)
    } else {
        format!("{:.1}GB", size as f64 / 1024.0 / 1024.0 / 1024.0)
    }
}

fn extract_package_name(filename: &str) -> String {
    // Extract package name from filename patterns like:
    // 20250301_193131_duckduckgo-search_pypi.txt
    // Format: {timestamp}_{package_name}_{source}.txt
    
    let re = Regex::new(r"\d{8}_\d{6}_([^_]+)_(\w+)\.txt$").unwrap();
    
    if let Some(captures) = re.captures(filename) {
        if captures.len() >= 2 {
            return captures[1].to_string();
        }
    }
    
    // Fallback - remove timestamp prefix if present
    let parts: Vec<&str> = filename.splitn(3, '_').collect();
    if parts.len() >= 3 && parts[0].len() == 8 && parts[1].len() == 6 {
        return parts[2].split('_').next().unwrap_or(filename).to_string();
    }
    
    filename.to_string()
}

/// Organizes output files into appropriate directories based on their type
///
/// This function analyzes output files, determines their package type, and
/// moves or copies them to the correct category directory. It helps maintain
/// a clean and structured output organization.
///
/// # Arguments
/// * `output_dir` - The base output directory to organize
///
/// # Returns
/// IO Result indicating success or failure of the organization process
pub fn organize_output(output_dir: &Path) -> std::io::Result<()> {
    // Create category directories if they don't exist
    for dir in ["local_repositories", "github_repos", "github_orgs", "pypi_packages", 
                "pypi_profiles", "crates", 
                "npm_packages", "go_packages"].iter() {
        fs::create_dir_all(output_dir.join(dir))?;
    }

    // Move any misplaced files to their correct directories
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            
            // Skip hidden files and directories
            if filename.starts_with('.') || path.is_dir() {
                continue;
            }
            
            // Determine correct directory based on filename
            let target_dir = if filename.contains("local") || filename.contains("_analysis.txt") {
                "local_repositories"
            } else if filename.contains("github_repo") || filename.contains("-")  && filename.ends_with("_github_repo.txt") {
                "github_repos"
            } else if filename.contains("github_org") {
                "github_orgs"
            } else if filename.contains("pypi_profile") {
                "pypi_profiles"
            } else if filename.contains("pypi") {
                "pypi_packages"
            } else if filename.contains("go") {
                "go_packages"
            } else if filename.contains("rust") || filename.contains("crate") {
                "crates"
            } else if filename.contains("npm") {
                "npm_packages"
            } else {
                continue; // Skip files that don't match any category
            };

            // Move file to correct directory if it's not already there
            let new_path = output_dir.join(target_dir).join(filename.to_string());
            if path != new_path {
                // Copy instead of move if it's a GitHub repo file that should be both in the root and in the github_repos directory
                if filename.contains("_github_repo") {
                    fs::copy(&path, &new_path)?;
                } else {
                    fs::rename(path, new_path)?;
                }
            }
        }
    }

    Ok(())
}

/// Generates an HTML index of all processed packages
///
/// This function creates a browsable HTML index of all processed packages,
/// allowing for easy navigation and access to package documentation through
/// a web browser.
///
/// # Arguments
/// * `output_dir` - The base output directory containing package files
///
/// # Returns
/// IO Result indicating success or failure of index generation
pub fn generate_index(output_dir: &Path) -> std::io::Result<()> {
    println!("Generating index of processed packages...");
    
    let mut packages = Vec::new();
    
    // The regex to extract information from filenames
    let filename_regex = Regex::new(r"(\d{8}_\d{6})_([^_]+)_(\w+)_(\w+)\.txt$").unwrap();
    
    // Scan all category directories
    for dir in ["local_repositories", "github_repos", "github_orgs", "pypi_packages", 
                "pypi_profiles", "rust_crates", 
                "npm_packages", "go_packages"].iter() {
        let dir_path = output_dir.join(dir);
        if !dir_path.exists() {
            continue;
        }
        
        for entry in fs::read_dir(&dir_path)? {
            let entry = entry?;
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }
            
            let filename = path.file_name().unwrap_or_default().to_string_lossy();
            
            // Extract information from both .txt and .md files
            if filename.ends_with(".txt") {
                if let Some(captures) = filename_regex.captures(&filename) {
                    let timestamp_str = &captures[1];
                    let name = captures[2].to_string();
                    let source = captures[3].to_string();
                    let file_type = captures[4].to_string();
                    
                    // Parse timestamp
                    let timestamp = if let Ok(dt) = NaiveDateTime::parse_from_str(
                        &format!("{}", timestamp_str), 
                        "%Y%m%d_%H%M%S"
                    ) {
                        DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc)
                    } else {
                        Utc::now() // Fallback to current time if parsing fails
                    };
                    
                    // Get file metadata
                    let metadata = fs::metadata(&path)?;
                    let file_size = metadata.len();
                    
                    // Extract description from file content (first few lines)
                    let description = extract_description(&path)?;
                    
                    packages.push(PackageInfo {
                        name,
                        version: None, // We'll try to extract this later
                        timestamp,
                        source: format!("{} ({})", source, file_type),
                        description,
                        file_path: path,
                        file_size,
                    });
                }
            } else if filename.ends_with(".md") {
                // For markdown files, extract info from content
                let name = filename.trim_end_matches(".md").to_string();
                
                // Get file metadata
                let metadata = fs::metadata(&path)?;
                let file_size = metadata.len();
                let modified = metadata.modified()?;
                
                // Convert to DateTime<Utc>
                let timestamp = DateTime::<Utc>::from(modified);
                
                // Extract description from file content
                let description = extract_description(&path)?;
                
                // Determine source based on directory
                let source = match *dir {
                    "local_repositories" => "local",
                    "github_repos" => "github_repo",
                    "github_orgs" => "github_org",
                    "pypi_packages" => "pypi",
                    "pypi_profiles" => "pypi_profile",
                    "rust_crates" => "crate",
                    "npm_packages" => "npm",
                    "go_packages" => "go",
                    _ => "unknown",
                };
                
                packages.push(PackageInfo {
                    name,
                    version: None,
                    timestamp,
                    source: source.to_string(),
                    description,
                    file_path: path,
                    file_size,
                });
            }
        }
    }
    
    // Sort packages by timestamp (newest first)
    packages.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    
    // Create index directory if it doesn't exist
    let index_dir = output_dir.join("_index");
    fs::create_dir_all(&index_dir)?;
    
    // Generate index file
    let index_path = index_dir.join("index.md");
    let mut file = File::create(&index_path)?;
    
    writeln!(file, "# Package Index\n")?;
    writeln!(file, "Generated: {}\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"))?;
    writeln!(file, "Total Packages: {}\n", packages.len())?;
    
    // Group by source type
    let mut sources = std::collections::HashMap::new();
    
    for package in &packages {
        let source = package.source.split_whitespace().next().unwrap_or("unknown");
        let entry = sources.entry(source.to_string()).or_insert_with(Vec::new);
        entry.push(package);
    }
    
    // Create sections for each source type
    for (source, pkgs) in &sources {
        writeln!(file, "## {} Packages ({})\n", source, pkgs.len())?;
        writeln!(file, "| Package | Processed | Size | Description |")?;
        writeln!(file, "|---------|-----------|------|-------------|")?;
        
        for package in pkgs.iter().take(100) { // Limit to 100 per section for readability
            let rel_path = pathdiff::diff_paths(&package.file_path, output_dir)
                .unwrap_or_else(|| package.file_path.clone());
            
            writeln!(
                file,
                "| [{}]({}) | {} | {} | {} |",
                package.name,
                rel_path.display().to_string().replace(' ', "%20"),
                package.timestamp.format("%Y-%m-%d %H:%M"),
                format_file_size(package.file_size),
                package.description.as_deref().unwrap_or(""),
            )?;
        }
        
        if pkgs.len() > 100 {
            writeln!(file, "| ... and {} more | | | |", pkgs.len() - 100)?;
        }
        
        writeln!(file)?;
    }
    
    // Also generate JSON index for programmatic access
    let json_index_path = index_dir.join("index.json");
    let json = serde_json::to_string_pretty(&packages)?;
    fs::write(json_index_path, json)?;
    
    // Create an HTML index for better browsing
    generate_html_index(output_dir, &packages, &index_dir)?;
    
    println!("[SUCCESS] Index generated at {}", index_path.display());
    
    Ok(())
}

/// Generate an HTML index file for better browsing of packages
fn generate_html_index(output_dir: &Path, packages: &[PackageInfo], index_dir: &Path) -> std::io::Result<()> {
    let html_path = index_dir.join("index.html");
    let mut file = File::create(&html_path)?;
    
    // Write HTML header
    writeln!(file, r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Package Index</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; line-height: 1.6; padding: 20px; max-width: 1200px; margin: 0 auto; }}
        h1, h2 {{ color: #2c3e50; }}
        table {{ border-collapse: collapse; width: 100%; margin-bottom: 20px; }}
        th, td {{ text-align: left; padding: 12px; }}
        th {{ background-color: #34495e; color: white; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
        tr:hover {{ background-color: #ddd; }}
        .search {{ margin-bottom: 20px; padding: 10px; width: 100%; }}
        .timestamp {{ white-space: nowrap; }}
        .size {{ white-space: nowrap; }}
        .package-count {{ color: #7f8c8d; font-size: 0.9em; }}
        .source-filter {{ margin-right: 10px; cursor: pointer; padding: 5px 10px; border-radius: 4px; }}
        .source-filter.active {{ background-color: #3498db; color: white; }}
        .filters {{ margin-bottom: 20px; }}
    </style>
</head>
<body>
    <h1>Package Index</h1>
    <p>Generated: {timestamp}</p>
    <p class="package-count">Total Packages: {total}</p>
    
    <input type="text" id="searchBox" class="search" placeholder="Search for packages...">
    
    <div class="filters">
        <button class="source-filter active" data-source="all">All</button>
"#, timestamp=Utc::now().format("%Y-%m-%d %H:%M:%S UTC"), total=packages.len())?;

    // Collect unique sources for filter buttons
    let mut sources = std::collections::HashSet::new();
    for package in packages {
        let source = package.source.split_whitespace().next().unwrap_or("unknown");
        sources.insert(source.to_string());
    }
    
    // Add source filter buttons
    for source in sources {
        writeln!(file, r#"        <button class="source-filter" data-source="{source}">{source}</button>"#, source=source)?;
    }

    // Write table headers
    writeln!(file, r#"    </div>
    
    <table id="packagesTable">
        <tr>
            <th>Package</th>
            <th>Source</th>
            <th>Processed</th>
            <th>Size</th>
            <th>Description</th>
        </tr>"#)?;

    // Write table rows for each package
    for package in packages {
        let rel_path = pathdiff::diff_paths(&package.file_path, output_dir)
            .unwrap_or_else(|| package.file_path.clone());
        
        let source = package.source.split_whitespace().next().unwrap_or("unknown");
        
        writeln!(file, r#"        <tr data-source="{source}">
            <td><a href="{path}">{name}</a></td>
            <td>{source}</td>
            <td class="timestamp">{timestamp}</td>
            <td class="size">{size}</td>
            <td>{description}</td>
        </tr>"#,
            source=source,
            path=rel_path.display().to_string().replace(' ', "%20"),
            name=package.name,
            timestamp=package.timestamp.format("%Y-%m-%d %H:%M"),
            size=format_file_size(package.file_size),
            description=package.description.as_deref().unwrap_or(""),
        )?;
    }

    // Write HTML footer with search and filter script
    writeln!(file, r#"    </table>

    <script>
        // Search functionality
        document.getElementById('searchBox').addEventListener('keyup', function() {{
            const search = this.value.toLowerCase();
            const rows = document.querySelectorAll('#packagesTable tr:not(:first-child)');
            
            rows.forEach(row => {{
                const text = row.textContent.toLowerCase();
                const display = text.includes(search) ? '' : 'none';
                
                // Only hide if also passes the current source filter
                if (display === 'none') {{
                    row.style.display = 'none';
                }} else {{
                    // Check against source filter
                    const activeSource = document.querySelector('.source-filter.active').dataset.source;
                    if (activeSource === 'all' || row.dataset.source === activeSource) {{
                        row.style.display = '';
                    }} else {{
                        row.style.display = 'none';
                    }}
                }}
            }});
        }});
        
        // Source filter functionality
        document.querySelectorAll('.source-filter').forEach(button => {{
            button.addEventListener('click', function() {{
                // Update active button
                document.querySelectorAll('.source-filter').forEach(btn => btn.classList.remove('active'));
                this.classList.add('active');
                
                const source = this.dataset.source;
                const rows = document.querySelectorAll('#packagesTable tr:not(:first-child)');
                
                // Apply filter
                rows.forEach(row => {{
                    if (source === 'all' || row.dataset.source === source) {{
                        // Also check against search filter
                        const search = document.getElementById('searchBox').value.toLowerCase();
                        const text = row.textContent.toLowerCase();
                        row.style.display = text.includes(search) ? '' : 'none';
                    }} else {{
                        row.style.display = 'none';
                    }}
                }});
            }});
        }});
    </script>
</body>
</html>"#)?;

    println!("[SUCCESS] HTML index generated at {}", html_path.display());
    
    Ok(())
}

// Enhance the description extraction function to handle more formats
fn extract_description(file_path: &Path) -> std::io::Result<Option<String>> {
    let file = File::open(file_path)?;
    let mut content = String::new();
    
    // Read only the first 8KB to look for description
    file.take(8192).read_to_string(&mut content)?;
    
    // Pattern matching for various description formats
    let patterns = [
        // Look for metadata descriptions
        ("Description:", "**Description**:"),
        ("Summary:", "**Summary**:"),
        ("## Description", "# Description"),
        ("## About", "# About"),
    ];
    
    for (pattern, alt_pattern) in &patterns {
        if let Some(pos) = content.find(pattern) {
            let text_after = &content[pos + pattern.len()..];
            let end_pos = text_after.find('\n').unwrap_or(text_after.len());
            let description = text_after[..end_pos].trim();
            
            if !description.is_empty() {
                // Truncate if too long
                if description.len() > 150 {
                    return Ok(Some(format!("{}...", &description[..147])));
                }
                return Ok(Some(description.to_string()));
            }
        } else if let Some(pos) = content.find(alt_pattern) {
            let text_after = &content[pos + alt_pattern.len()..];
            let end_pos = text_after.find('\n').unwrap_or(text_after.len());
            let description = text_after[..end_pos].trim();
            
            if !description.is_empty() {
                // Truncate if too long
                if description.len() > 150 {
                    return Ok(Some(format!("{}...", &description[..147])));
                }
                return Ok(Some(description.to_string()));
            }
        }
    }
    
    // Parse bullet points for description
    let bullet_patterns = [
        "- **Description**:", 
        "- **Summary**:",
        "- Description:"
    ];
    
    for pattern in &bullet_patterns {
        if let Some(pos) = content.find(pattern) {
            let text_after = &content[pos + pattern.len()..];
            let end_pos = text_after.find('\n').unwrap_or(text_after.len());
            let description = text_after[..end_pos].trim();
            
            if !description.is_empty() {
                // Truncate if too long
                if description.len() > 150 {
                    return Ok(Some(format!("{}...", &description[..147])));
                }
                return Ok(Some(description.to_string()));
            }
        }
    }
    
    // Look for first non-empty line after a heading if nothing else works
    for line in content.lines().skip_while(|l| !l.starts_with('#')) {
        if line.starts_with('#') {
            continue;
        }
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('-') && !trimmed.starts_with('*') {
            // Truncate if too long
            if trimmed.len() > 150 {
                return Ok(Some(format!("{}...", &trimmed[..147])));
            }
            return Ok(Some(trimmed.to_string()));
        }
    }
    
    Ok(None)
} 