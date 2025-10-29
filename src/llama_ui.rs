use std::path::{Path, PathBuf};
use std::time::Duration;
use colored::{Color, Colorize, ColoredString};
use dialoguer::{Select, Input, Confirm, theme::ColorfulTheme};
use dialoguer::console::{StyledObject, Term};
use indicatif::{ProgressBar, ProgressStyle};
use llamapackageservice::{
    Config,
    error::{ProcessorError, Result},
    processors::{github, pypi, npm, crates},
};
use tokio::time::sleep;

/// The main menu options for the Llama UI
#[derive(Debug, Clone, Copy)]
pub enum LlamaMenuOption {
    ProcessGithub,
    ProcessPyPI,
    ProcessNPM,
    ProcessCrate,
    ViewOutput,
    Settings,
    Exit,
}

/// A Llama-themed user interface for the llama-package-service application
pub struct LlamaUI {
    theme: ColorfulTheme,
}

impl LlamaUI {
    /// Creates a new LlamaUI with a custom theme
    pub fn new() -> Self {
        let mut theme = ColorfulTheme::default();
        // Convert to plain strings first
        let prompt_str = "[PROMPT] ".to_string();
        let success_str = "[SUCCESS] ".to_string();
        let error_str = "[ERROR] ".to_string();
        
        // Use dialoguer's console styling instead of colored crate
        theme.prompt_prefix = dialoguer::console::style(prompt_str).cyan().bright();
        theme.success_prefix = dialoguer::console::style(success_str).green().bright();
        theme.error_prefix = dialoguer::console::style(error_str).red().bright();
        
        Self { theme }
    }

    /// Displays the Llama ASCII logo
    pub fn display_logo(&self) {
        let logo = r#"
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣀⣀⣀⣀⣀⣀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣴⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣦⡀⠀⠀⠀⠀⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣴⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣄⠀⠀⠀⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣠⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⠀⠀⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢀⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡆⠀⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⢠⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡀⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣰⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⠀⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣼⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡄⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣾⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣷⠀⠀⠀
        ⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣸⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡆⠀⠀
        ⠀⠀⠀⢀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣀⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⣀⣀
        ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿
        ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠘⣿⣿⣿
        ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⡇⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⠀⣿⣿⣿
        ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣧⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣤⣿⣿⣿
        ⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿⣿
        ⠈⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉⠉
        "#;
        
        println!("{}", logo.bright_yellow());
        println!("{}", "Llama Package Service - The Llama Search Tool".bright_magenta().bold());
        println!("{}", "Convert code repositories to text for analysis".bright_blue());
        println!("{}", "==================================================".bright_cyan());
    }

    /// Shows a welcome message to the user
    pub fn show_welcome(&self) {
        println!("\n{}", "Welcome to Llama Package Service - The Llama Search Tool!".bright_magenta().bold());
        println!("{}", "This tool helps you convert code repositories into text format.".bright_blue());
        println!("{}", "Let's get started!".bright_green());
    }

    /// Shows the main menu and returns the selected option
    pub fn show_main_menu(&self) -> Result<LlamaMenuOption> {
        let items = vec![
            "1. Process GitHub Repository",
            "2. Process PyPI Package",
            "3. Process NPM Package",
            "4. Process Rust Crate",
            "5. View Output Directory",
            "6. Settings",
            "7. Exit",
        ];
        
        // Print menu options manually
        println!("\n{}", "Menu Options:".bright_cyan().bold());
        for item in &items {
            println!("{}", item);
        }
        
        // Use Input instead of Select to avoid terminal height issues
        let selection = Input::<String>::with_theme(&self.theme)
            .with_prompt("Enter your choice (1-7)")
            .validate_with(|input: &String| -> std::result::Result<(), &str> {
                match input.trim().parse::<usize>() {
                    Ok(n) if n >= 1 && n <= 7 => Ok(()),
                    _ => Err("Please enter a number between 1 and 7"),
                }
            })
            .interact()
            .map_err(|e| ProcessorError::Message(format!("Menu selection error: {}", e)))?;
        
        // Parse the selection
        let selection = selection.trim().parse::<usize>()
            .map_err(|_| ProcessorError::Message("Invalid menu option".to_string()))?;
        
        match selection {
            1 => Ok(LlamaMenuOption::ProcessGithub),
            2 => Ok(LlamaMenuOption::ProcessPyPI),
            3 => Ok(LlamaMenuOption::ProcessNPM),
            4 => Ok(LlamaMenuOption::ProcessCrate),
            5 => Ok(LlamaMenuOption::ViewOutput),
            6 => Ok(LlamaMenuOption::Settings),
            7 => Ok(LlamaMenuOption::Exit),
            _ => Err(ProcessorError::Message("Invalid menu option".to_string())),
        }
    }

    /// Gets a package URL from the user
    pub fn get_package_url(&self, package_type: &str) -> Result<String> {
        let prompt = format!("Enter {} URL:", package_type);
        
        let url = Input::with_theme(&self.theme)
            .with_prompt(&prompt)
            .validate_with(|input: &String| -> std::result::Result<(), &str> {
                if input.trim().is_empty() {
                    return Err("URL cannot be empty");
                }
                Ok(())
            })
            .interact()
            .map_err(|e| ProcessorError::Message(format!("Input error: {}", e)))?;
            
        Ok(url)
    }

    /// Asks the user to confirm an action
    pub fn confirm(&self, message: &str) -> Result<bool> {
        let confirm = Confirm::with_theme(&self.theme)
            .with_prompt(message)
            .default(true)
            .interact()
            .map_err(|e| ProcessorError::Message(format!("Confirmation error: {}", e)))?;
            
        Ok(confirm)
    }

    /// Gets an output directory from the user
    pub fn select_output_dir(&self, default_dir: &Path) -> Result<PathBuf> {
        let dir_str = default_dir.to_string_lossy();
        
        let prompt = format!("Output directory [default: {}]:", dir_str);
        
        let input = Input::<String>::with_theme(&self.theme)
            .with_prompt(&prompt)
            .default(dir_str.to_string())
            .interact()
            .map_err(|e| ProcessorError::Message(format!("Input error: {}", e)))?;
            
        if input.trim().is_empty() {
            Ok(default_dir.to_path_buf())
        } else {
            Ok(PathBuf::from(input))
        }
    }

    /// Creates a progress bar for general operations
    pub fn create_progress_bar(&self) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
                .template("{spinner:.green} {msg}")
                .unwrap()
        );
        
        pb.enable_steady_tick(Duration::from_millis(120));
        
        pb
    }

    /// Creates a progress bar for file operations
    pub fn create_file_progress_bar(&self, total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta}) {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        
        pb
    }

    /// Prints a success message
    pub fn print_success(&self, message: &str) {
        println!("{} {}", "[SUCCESS]".bright_green(), message);
    }

    /// Prints an info message
    pub fn print_info(&self, message: &str) {
        println!("{} {}", "[INFO]".bright_blue(), message);
    }

    /// Prints a warning message
    pub fn print_warning(&self, message: &str) {
        println!("{} {}", "[WARNING]".bright_yellow(), message);
    }

    /// Prints an error message
    pub fn print_error(&self, message: &str) {
        println!("{} {}", "[ERROR]".bright_red(), message);
    }

    /// Prints a section header
    pub fn print_section(&self, title: &str) {
        println!("\n{} {} {}", 
            "==>".bright_cyan(),
            title.bright_magenta().bold(),
            "<==".bright_cyan()
        );
        println!("{}", "=".repeat(title.len() + 10).bright_blue());
    }

    /// Gets a GitHub token from the user
    pub fn get_github_token(&self) -> Result<String> {
        let token = Input::<String>::with_theme(&self.theme)
            .with_prompt("Enter GitHub token (to avoid rate limiting)")
            .allow_empty(true)
            .interact()
            .map_err(|e| ProcessorError::Message(format!("Input error: {}", e)))?;
            
        Ok(token)
    }

    /// Shows a progress spinner while executing a function
    pub async fn show_progress_with_message<F, T>(&self, message: &str, f: F) -> Result<T>
    where
        F: FnOnce() -> Result<T>,
    {
        let pb = self.create_progress_bar();
        pb.set_message(message.to_string());
        
        let result = f()?;
        
        pb.finish_with_message(format!("{} Done!", message));
        sleep(Duration::from_millis(500)).await;
        
        Ok(result)
    }

    /// Runs the UI in an interactive loop
    pub async fn run(&self, config: &Config) -> Result<()> {
        self.display_logo();
        self.show_welcome();
        
        let output_dir = config.output_dir.clone();
        
        loop {
            match self.show_main_menu()? {
                LlamaMenuOption::ProcessGithub => {
                    let url = self.get_package_url("GitHub")?;
                    let pb = self.create_progress_bar();
                    
                    match github::process_github_url(&url, &output_dir, &pb).await {
                        Ok(_) => self.print_success(&format!("Successfully processed GitHub URL: {}", url)),
                        Err(e) => self.print_error(&format!("Error processing GitHub URL: {}", e)),
                    }
                    
                    sleep(Duration::from_secs(1)).await;
                },
                LlamaMenuOption::ProcessPyPI => {
                    let url = self.get_package_url("PyPI")?;
                    let pb = self.create_progress_bar();
                    
                    match pypi::process_pypi_url(&url, &output_dir, &pb).await {
                        Ok(_) => self.print_success(&format!("Successfully processed PyPI URL: {}", url)),
                        Err(e) => self.print_error(&format!("Error processing PyPI URL: {}", e)),
                    }
                    
                    sleep(Duration::from_secs(1)).await;
                },
                LlamaMenuOption::ProcessNPM => {
                    let url = self.get_package_url("NPM")?;
                    let pb = self.create_progress_bar();
                    
                    match npm::process_npm_package(&url, &output_dir, &pb).await {
                        Ok(_) => self.print_success(&format!("Successfully processed NPM URL: {}", url)),
                        Err(e) => self.print_error(&format!("Error processing NPM URL: {}", e)),
                    }
                    
                    sleep(Duration::from_secs(1)).await;
                },
                LlamaMenuOption::ProcessCrate => {
                    let url = self.get_package_url("Rust Crate")?;
                    let pb = self.create_progress_bar();
                    
                    match crates::process_crate(&url, &output_dir, &pb).await {
                        Ok(_) => self.print_success(&format!("Successfully processed Crate URL: {}", url)),
                        Err(e) => self.print_error(&format!("Error processing Crate URL: {}", e)),
                    }
                    
                    sleep(Duration::from_secs(1)).await;
                },
                LlamaMenuOption::ViewOutput => {
                    self.print_section("Output Directory");
                    
                    match std::fs::read_dir(&output_dir) {
                        Ok(entries) => {
                            let mut files: Vec<String> = Vec::new();
                            
                            for entry in entries {
                                if let Ok(entry) = entry {
                                    files.push(entry.file_name().to_string_lossy().to_string());
                                }
                            }
                            
                            if files.is_empty() {
                                self.print_info("No files in output directory yet.");
                            } else {
                                self.print_info(&format!("Output directory: {}", output_dir.display()));
                                for file in files {
                                    println!("  - {}", file);
                                }
                            }
                        },
                        Err(e) => {
                            self.print_error(&format!("Error reading output directory: {}", e));
                        }
                    }
                    
                    sleep(Duration::from_secs(2)).await;
                },
                LlamaMenuOption::Settings => {
                    self.print_section("Settings");
                    self.print_info("Settings functionality is not yet implemented.");
                    sleep(Duration::from_secs(2)).await;
                },
                LlamaMenuOption::Exit => {
                    self.print_success("Thank you for using Llama Package Service - The Llama Search Tool!");
                    break;
                }
            }
        }
        
        Ok(())
    }
} 