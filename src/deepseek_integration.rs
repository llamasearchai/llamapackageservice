use std::process::Command;
use std::io;

pub fn ask_deepseek(query: &str) -> io::Result<String> {
    // Run the "ollama" command with your deepseek model.
    // Ensure that "ollama" is installed and deepseek-r1:70b is available.
    let output = Command::new("ollama")
        .args(&["run", "deepseek-r1:70b", "--prompt", query])
        .output()?;
    let response = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(response)
} 