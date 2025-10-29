use colored::*;
use dialoguer::{Input, Select, Confirm, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::Write;
use std::path::PathBuf;
use std::time::Duration;
use crate::error::{ProcessorError, Result};

pub enum MenuOption {
    ProcessPackage,
    RunTests,
    ViewOutput,
    Configure,
    ConfigureGitHubToken,
    Exit,
}

pub struct UI {
    theme: ColorfulTheme,
}

impl UI {
    pub fn new() -> Self {
        Self {
            theme: ColorfulTheme::default(),
        }
    }

    pub fn show_main_menu(&self) -> Result<MenuOption> {
        let selections = &[
            "Process Package/Repository",
            "Run Test Suite",
            "View Output Files",
            "Configure Settings",
            "Configure GitHub Token",
            "Exit",
        ];

        match Select::with_theme(&self.theme)
            .with_prompt("\n[MENU] LlamaSearch Menu")
            .items(selections)
            .default(0)
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))?
        {
            0 => Ok(MenuOption::ProcessPackage),
            1 => Ok(MenuOption::RunTests),
            2 => Ok(MenuOption::ViewOutput),
            3 => Ok(MenuOption::Configure),
            4 => Ok(MenuOption::ConfigureGitHubToken),
            _ => Ok(MenuOption::Exit),
        }
    }

    pub fn get_package_details(&self) -> Result<(String, String)> {
        // Let user choose package type
        let types = [
            "github-repository",
            "github-organization",
            "pypi-package",
            "rust-crate",
        ];
        let selection = Select::with_theme(&self.theme)
            .with_prompt("Select package type")
            .items(&types)
            .default(0)
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))?;
        let pkg_type = types[selection].to_string();
        
        // Get package URL
        let pkg_url = Input::<String>::with_theme(&self.theme)
            .with_prompt("Enter package URL")
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))?;
        Ok((pkg_type, pkg_url))
    }

    pub fn get_query(&self, prompt: &str) -> Result<String> {
        Input::<String>::with_theme(&self.theme)
            .with_prompt(prompt)
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))
    }

    pub fn confirm(&self, prompt: &str) -> Result<bool> {
        Confirm::with_theme(&self.theme)
            .with_prompt(prompt)
            .default(true)
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))
    }

    pub fn select_output_dir(&self) -> Result<PathBuf> {
        let default_dir = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("llamasearch_output");
            
        if self.confirm(&format!("Use default output directory? ({})", default_dir.display()))? {
            return Ok(default_dir);
        }
        
        let custom_path = Input::<String>::with_theme(&self.theme)
            .with_prompt("Enter output directory path")
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))?;
            
        Ok(PathBuf::from(custom_path))
    }

    pub fn create_progress_bar(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
                .tick_strings(&[
                    "⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"
                ])
        );
        pb.enable_steady_tick(Duration::from_millis(100));
        pb.set_message(message.to_string());
        pb
    }

    pub fn create_file_progress_bar(&self, total_size: u64, message: &str) -> ProgressBar {
        let pb = ProgressBar::new(total_size);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] {bar:40.cyan/blue} {bytes}/{total_bytes} ({eta}) {msg}")
                .unwrap()
                .progress_chars("█▉▊▋▌▍▎▏  ")
        );
        pb.set_message(message.to_string());
        pb
    }

    pub fn print_banner(&self) {
        let banner = format!(
            "{}\n{}\n{}\n{}\n{}",
            "         __      _                     ".blue(),
            "       /\\ \\__  /\\ \\__                ".blue(),
            "  _____\\ \\ ,_\\/  \\ ,_\\   LlamaSearch    ".bold().green(),
            "_______\\ \\ \\/____\\ \\ \\/___ Package Analyzer".bold().green(),
            "         \\ \\__\\   \\ \\__\\                ".blue()
        );
        println!("{}", banner);
    }

    pub fn print_info(&self, message: &str) {
        println!("{}", message.green());
    }

    pub fn print_warning(&self, message: &str) {
        println!("{}", message.yellow());
    }

    pub fn print_error(&self, message: &str) {
        eprintln!("{}", message.red());
    }

    pub fn print_section(&self, title: &str) {
        let width = title.len() + 4;
        let border = "=".repeat(width);
        
        println!("\n{}", border.bright_blue());
        println!("  {}", title.bright_white().bold());
        println!("{}\n", border.bright_blue());
    }

    pub fn get_github_token(&self) -> Result<String> {
        let token = Input::<String>::with_theme(&self.theme)
            .with_prompt("Enter your GitHub token")
            .allow_empty(true)
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))?;
            
        Ok(token)
    }
    
    pub fn show_progress_with_message<F>(&self, message: &str, f: F) -> Result<()> 
    where
        F: FnOnce() -> Result<()>
    {
        let pb = self.create_progress_bar(message);
        let result = f();
        pb.finish_with_message(format!("{} - Complete", message));
        result
    }
    
    pub fn select_from_list<T: AsRef<str>>(&self, prompt: &str, items: &[T]) -> Result<usize> {
        Select::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(items)
            .default(0)
            .interact()
            .map_err(|e| ProcessorError::new(&e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ui_creation() {
        let ui = UI::new();
        // Just ensure we can create a UI instance without panic
        assert!(ui.theme.highlight_style.is_some());
    }
} 