use colored::*;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use crate::error::ProcessorError;
use std::io::Write;

/// Prints a colorful banner at the start of the CLI.
pub fn print_banner() {
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

/// Prompts the user for input using an interactive dialog.
pub fn prompt_user(prompt: &str) -> String {
    Input::new()
        .with_prompt(prompt)
        .interact_text()
        .unwrap_or_else(|_| String::from("default_package"))
}

/// (Optional) Helper to print an info message.
pub fn print_info(message: &str) {
    println!("{}", message.green());
}

/// (Optional) Helper to print a warning message.
pub fn print_warning(message: &str) {
    println!("{}", message.yellow());
}

/// (Optional) Helper to print an error message.
pub fn print_error(message: &str) {
    eprintln!("{}", message.red());
}

#[derive(Debug)]
pub enum MenuOption {
    ProcessPackage,
    RunTests,
    ViewOutput,
    Configure,
    ConfigureGitHubToken,
    Exit,
}

pub fn show_main_menu() -> MenuOption {
    let selections = &[
        "Process Package/Repository",
        "Run Test Suite",
        "View Output Files",
        "Configure Settings",
        "Configure GitHub Token",
        "Exit",
    ];

    match dialoguer::Select::new()
        .with_prompt("\n[MENU] LlamaSearch Menu")
        .items(selections)
        .default(0)
        .interact()
        .unwrap()
    {
        0 => MenuOption::ProcessPackage,
        1 => MenuOption::RunTests,
        2 => MenuOption::ViewOutput,
        3 => MenuOption::Configure,
        4 => MenuOption::ConfigureGitHubToken,
        _ => MenuOption::Exit,
    }
}

pub fn get_package_details() -> Result<(String, String), ProcessorError> {
    // Let user choose package type (e.g., pypi-package, rust-crate, github-repository, github-organization)
    let types = [
        "github-repository",
        "github-organization",
        "pypi-package",
        "rust-crate",
    ];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Select package type")
        .items(&types)
        .default(0)
        .interact()
        .map_err(|e| ProcessorError::new(&e.to_string()))?;
    let pkg_type = types[selection].to_string();
    
    // Get package URL
    let pkg_url = Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter package URL")
        .interact()
        .map_err(|e| ProcessorError::new(&e.to_string()))?;
    Ok((pkg_type, pkg_url))
}

pub fn get_deepseek_query() -> Result<String, ProcessorError> {
    Input::<String>::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter your query for Deepseek")
        .interact()
        .map_err(|e| ProcessorError::new(&e.to_string()))
}

pub fn get_package_link() -> Result<String, ProcessorError> {
    Input::new()
        .with_prompt("Enter package URL to process")
        .interact()
        .map_err(|e| ProcessorError::new(&e.to_string()))
} 