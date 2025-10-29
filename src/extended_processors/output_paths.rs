use std::path::{Path, PathBuf};
use colored::Colorize;

#[derive(Debug)]
pub struct OutputPaths {
    pub base_dir: PathBuf,
    pub github_repos_dir: PathBuf,
    pub github_orgs_dir: PathBuf,
    pub pypi_packages_dir: PathBuf,
    pub pypi_profiles_dir: PathBuf,
    pub go_packages_dir: PathBuf,
    pub rust_packages_dir: PathBuf,
    pub npm_packages_dir: PathBuf,
}

impl OutputPaths {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        let base = base_dir.as_ref().to_path_buf();
        OutputPaths {
            base_dir: base.clone(),
            github_repos_dir: base.join("github_repos"),
            github_orgs_dir: base.join("github_orgs"),
            pypi_packages_dir: base.join("pypi_packages"),
            pypi_profiles_dir: base.join("pypi_profiles"),
            go_packages_dir: base.join("go_packages"),
            rust_packages_dir: base.join("rust_packages"),
            npm_packages_dir: base.join("npm_packages"),
        }
    }

    pub fn create_all_dirs(&self) -> std::io::Result<()> {
        println!("{}", "\n🗂️  Creating output directories...".bright_cyan());
        
        let dirs = [
            (&self.base_dir, "Base"),
            (&self.github_repos_dir, "GitHub Repos"),
            (&self.github_orgs_dir, "GitHub Organizations"),
            (&self.pypi_packages_dir, "PyPI Packages"),
            (&self.pypi_profiles_dir, "PyPI Profile Packages"),
            (&self.go_packages_dir, "Go Packages"),
            (&self.rust_packages_dir, "Rust Crates"),
            (&self.npm_packages_dir, "NPM Packages"),
        ];

        for (dir, name) in dirs {
            std::fs::create_dir_all(dir)?;
            println!("  [OK] Created: {}", name.bright_green());
        }
        
        println!();
        Ok(())
    }
} 