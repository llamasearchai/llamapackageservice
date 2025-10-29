use crate::error::ProcessorError;
use crate::processors::common::{self, setup_progress_style};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use std::fs;

/// Represents a Jupyter notebook cell
#[derive(Debug)]
struct NotebookCell {
    cell_type: String,
    source: Vec<String>,
    outputs: Option<Vec<Value>>,
}

/// Process a cookbook repository that contains Jupyter notebooks
pub async fn process_cookbook_repo(
    repo_url: &str, 
    output_dir: &Path,
    pb: &ProgressBar
) -> Result<(), ProcessorError> {
    setup_progress_style(pb);
    pb.set_message("Processing cookbook repository...");

    // Download the repository
    let archive_bytes = common::download_github_repo(repo_url, pb).await?;
    
    // Create temporary directory for extraction
    let temp_dir = tempfile::tempdir()
        .map_err(|e| ProcessorError::IO(e))?;
    
    // Extract the archive
    common::extract_zip_archive(&archive_bytes, temp_dir.path(), pb)?;
    
    // Find all Jupyter notebooks recursively
    pb.set_message("Finding Jupyter notebooks...");
    let notebooks = find_jupyter_notebooks(temp_dir.path())?;
    
    if notebooks.is_empty() {
        return Err(ProcessorError::Processing("No Jupyter notebooks found".to_string()));
    }
    
    pb.set_length(notebooks.len() as u64);
    pb.set_style(ProgressStyle::default_bar()
        .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} notebooks processed {msg}")
        .unwrap());
    
    // Process each notebook
    let mut master_index = String::new();
    master_index.push_str("# Cookbook Contents\n\n");
    
    for (i, notebook_path) in notebooks.iter().enumerate() {
        pb.set_position(i as u64);
        pb.set_message(format!("Processing {}", notebook_path.display()));
        
        let notebook_content = process_notebook(notebook_path)?;
        master_index.push_str(&notebook_content);
    }
    
    // Save the processed content
    let repo_name = repo_url.split('/').last().unwrap_or("unknown");
    let timestamp = time::OffsetDateTime::now_utc().unix_timestamp();
    let file_name = format!("{}_cookbook_{}.md", repo_name, timestamp);
    let file_path = output_dir.join(file_name);
    
    fs::write(file_path, master_index)
        .map_err(|e| ProcessorError::IO(e))?;
    
    pb.finish_with_message(format!("✨ Processed {} notebooks", notebooks.len()));
    Ok(())
}

/// Find all Jupyter notebooks in a directory recursively
fn find_jupyter_notebooks(dir: &Path) -> Result<Vec<PathBuf>, ProcessorError> {
    let mut notebooks = Vec::new();
    
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() && entry.path().extension().map_or(false, |ext| ext == "ipynb") {
            notebooks.push(entry.path().to_owned());
        }
    }
    
    Ok(notebooks)
}

/// Process a single Jupyter notebook
fn process_notebook(path: &Path) -> Result<String, ProcessorError> {
    let content = fs::read_to_string(path)
        .map_err(|e| ProcessorError::IO(e))?;
    
    let notebook: Value = serde_json::from_str(&content)
        .map_err(|e| ProcessorError::Processing(format!("Invalid notebook JSON: {}", e)))?;
    
    let mut output = String::new();
    
    // Add notebook title (from filename or first heading)
    let title = path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled Notebook");
    
    output.push_str(&format!("\n## {}\n\n", title));
    
    // Process cells
    if let Some(cells) = notebook["cells"].as_array() {
        for cell in cells {
            if let Some(cell_type) = cell["cell_type"].as_str() {
                match cell_type {
                    "markdown" => {
                        if let Some(source) = cell["source"].as_array() {
                            for line in source {
                                if let Some(line_str) = line.as_str() {
                                    output.push_str(line_str);
                                }
                            }
                            output.push_str("\n\n");
                        }
                    },
                    "code" => {
                        if let Some(source) = cell["source"].as_array() {
                            output.push_str("```python\n");
                            for line in source {
                                if let Some(line_str) = line.as_str() {
                                    output.push_str(line_str);
                                }
                            }
                            output.push_str("\n```\n\n");
                        }
                        
                        // Add outputs if available
                        if let Some(outputs) = cell["outputs"].as_array() {
                            for output_cell in outputs {
                                if let Some(text) = output_cell["text"].as_array() {
                                    output.push_str("Output:\n```\n");
                                    for line in text {
                                        if let Some(line_str) = line.as_str() {
                                            output.push_str(line_str);
                                        }
                                    }
                                    output.push_str("\n```\n\n");
                                }
                            }
                        }
                    },
                    _ => continue,
                }
            }
        }
    }
    
    Ok(output)
} 