use reqwest;
use std::fs::{self, File};
use std::io::{self, Write};
use zip::ZipArchive;
use tokio;

/// Downloads the repository archive from the given URL and writes it to the specified output path.
pub async fn download_repo(url: &str, output_path: &str) -> io::Result<()> {
    let response = reqwest::get(url).await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Request error: {}", e))
    })?.bytes().await.map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("Bytes error: {}", e))
    })?;
    
    let mut file = File::create(output_path)?;
    file.write_all(&response)?;
    Ok(())
}

/// Extracts the ZIP archive at zip_path into the directory extract_to.
pub fn extract_zip(zip_path: &str, extract_to: &str) -> io::Result<()> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    archive.extract(extract_to)?;
    Ok(())
}

/// Iterates through the repository files, concatenates their paths and contents,
/// and writes the final output file.
pub async fn process_repo_content(repo_path: &str, output_file: &str) -> io::Result<()> {
    // Collect all file data in a single string
    let mut content = String::new();

    // Recursively walk directories if needed
    fn visit_dirs(dir: &std::path::Path, buff: &mut String) -> io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, buff)?;
                } else if path.is_file() {
                    if let Ok(file_content) = fs::read_to_string(&path) {
                        buff.push_str(&format!("File: {}\n\n{}\n\n", path.display(), file_content));
                    }
                }
            }
        }
        Ok(())
    }

    visit_dirs(std::path::Path::new(repo_path), &mut content)?;
    tokio::fs::write(output_file, content).await?;
    Ok(())
}
