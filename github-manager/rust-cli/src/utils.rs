use std::path::{Path, PathBuf};
use std::fs;
use humansize::{format_size, BINARY};

use crate::error::{Error, Result};

pub fn expand_tilde(path: &Path) -> PathBuf {
    if path.starts_with("~") {
        if let Some(home) = dirs::home_dir() {
            home.join(path.strip_prefix("~").unwrap())
        } else {
            path.to_path_buf()
        }
    } else {
        path.to_path_buf()
    }
}

pub fn format_file_size(size: u64) -> String {
    format_size(size, BINARY)
}

pub fn ensure_dir_exists(path: &Path) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| Error::Io(e))?;
    }
    Ok(())
}

pub fn truncate_string(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

pub fn get_file_extension(path: &Path) -> Option<&str> {
    path.extension()?.to_str()
}

pub fn is_binary_file(path: &Path) -> Result<bool> {
    let data = fs::read(path)
        .map_err(|e| Error::Io(e))?;
    
    // Simple heuristic: check for null bytes in first 8KB
    let check_len = std::cmp::min(data.len(), 8192);
    Ok(data[..check_len].contains(&0))
}

pub fn strip_ansi_codes(s: &str) -> String {
    // Simple ANSI escape code remover - removes common ANSI color codes
    let mut result = s.to_string();
    
    // Remove common ANSI escape sequences
    for pattern in &[
        "\x1b[0m", "\x1b[1m", "\x1b[2m", "\x1b[3m", "\x1b[4m", "\x1b[5m", "\x1b[6m", "\x1b[7m",
        "\x1b[30m", "\x1b[31m", "\x1b[32m", "\x1b[33m", "\x1b[34m", "\x1b[35m", "\x1b[36m", "\x1b[37m",
        "\x1b[40m", "\x1b[41m", "\x1b[42m", "\x1b[43m", "\x1b[44m", "\x1b[45m", "\x1b[46m", "\x1b[47m",
    ] {
        result = result.replace(pattern, "");
    }
    
    // Remove more complex ANSI sequences
    let mut cleaned = String::new();
    let mut in_escape = false;
    
    for ch in result.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape && ch == 'm' {
            in_escape = false;
        } else if !in_escape {
            cleaned.push(ch);
        }
    }
    
    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_truncate_string() {
        assert_eq!(truncate_string("hello", 10), "hello");
        assert_eq!(truncate_string("hello world", 8), "hello...");
    }
    
    #[test]
    fn test_format_file_size() {
        assert_eq!(format_file_size(1024), "1 KiB");
        assert_eq!(format_file_size(1024 * 1024), "1 MiB");
    }
}