use colored::*;
use std::{error::Error, fs, io::Write, path::Path};

/// Organizes files by reading all files in the given input directory,
/// concatenating their contents (with file information), and writing the result to a combined output file.
pub fn organize_files(input_dir: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    // Create the output directory if it does not exist.
    if !Path::new(output_dir).exists() {
        fs::create_dir_all(output_dir)?;
        println!(
            "{}",
            format!("Created output directory: {}", output_dir).green()
        );
    }
    let mut combined_content = String::new();

    if Path::new(input_dir).exists() {
        for entry in fs::read_dir(input_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path)?;
                combined_content.push_str(&format!("--- File: {} ---\n", path.display()));
                combined_content.push_str(&content);
                combined_content.push_str("\n\n");
            }
        }
    } else {
        println!(
            "{}",
            format!(
                "Input directory '{}' does not exist. Skipping file organization.",
                input_dir
            )
            .yellow()
        );
    }
    let output_file_path = format!("{}/combined_output.txt", output_dir);
    let mut file = fs::File::create(&output_file_path)?;
    file.write_all(combined_content.as_bytes())?;
    println!(
        "{}",
        format!("Combined file written to: {}", output_file_path).green()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_organize_files() {
        // Create temporary input directory with sample files.
        let temp_in = tempdir().unwrap();
        let temp_out = tempdir().unwrap();
        let file1_path = temp_in.path().join("file1.txt");
        fs::write(&file1_path, "Content of file 1").unwrap();
        let file2_path = temp_in.path().join("file2.txt");
        fs::write(&file2_path, "Content of file 2").unwrap();

        // Run the file organization.
        let result = organize_files(
            temp_in.path().to_str().unwrap(),
            temp_out.path().to_str().unwrap(),
        );
        assert!(result.is_ok());
        let output_file_path = temp_out.path().join("combined_output.txt");
        let output_content = fs::read_to_string(output_file_path).unwrap();
        assert!(output_content.contains("Content of file 1"));
        assert!(output_content.contains("Content of file 2"));
    }
} 