use assert_cmd::prelude::*;
use std::process::Command;
use tempfile::TempDir;
use std::fs;

// Verify the CLI can process a local directory path even when the provided input contains a trailing space
#[test]
fn process_local_path_with_trailing_space() {
    // Create a temporary project directory with a simple file
    let temp_root = TempDir::new().expect("failed to create temp root");
    let project_dir = temp_root.path().join("OpenResearcher");
    fs::create_dir_all(&project_dir).expect("failed to create project dir");
    fs::write(project_dir.join("README.md"), "# Test Project\n\nSome content.")
        .expect("failed to write file");

    // Output directory
    let out_dir = TempDir::new().expect("failed to create output dir");

    // Build an input string with a trailing space
    let input_with_space = format!("{} ", project_dir.display());

    // Run the binary with --url set to the path with trailing space
    let mut cmd = Command::cargo_bin("llamapackageservice").expect("binary not found");
    let assert = cmd
        .arg("--url")
        .arg(&input_with_space)
        .arg("--output")
        .arg(out_dir.path())
        .assert();

    // Should succeed
    assert.success();

    // Verify that local_repositories output exists and contains at least one file
    let lr = out_dir.path().join("local_repositories");
    assert!(lr.exists() && lr.is_dir(), "local_repositories directory missing");
    let entries: Vec<_> = fs::read_dir(&lr).expect("read_dir failed").collect();
    assert!(!entries.is_empty(), "no analysis files produced in local_repositories");
}


