use std::path::PathBuf;

/// Normalize a user-provided local path string into a canonical PathBuf suitable for processing.
///
/// - Trims leading/trailing ASCII and Unicode whitespace
/// - Strips surrounding single or double quotes if present
/// - Expands a leading '~' to the HOME directory when possible
pub fn normalize_user_input_path(input: &str) -> PathBuf {
    let trimmed = input.trim();

    // Remove surrounding quotes if both ends match
    let unquoted = if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        &trimmed[1..trimmed.len().saturating_sub(1)]
    } else {
        trimmed
    };

    // Expand ~ to home dir
    if unquoted.starts_with('~') {
        // Handle cases: "~" or "~/sub/path"
        let without_tilde = unquoted.strip_prefix("~/").or_else(|| unquoted.strip_prefix("~")).unwrap_or(unquoted);
        if let Some(home) = std::env::var_os("HOME") {
            let mut buf = PathBuf::from(home);
            if !without_tilde.is_empty() {
                buf.push(without_tilde);
            }
            return buf;
        }
    }

    PathBuf::from(unquoted)
}

/// Normalize a value that may be a URL or a local path.
///
/// - If it looks like a local path, returns a normalized absolute/clean string path
/// - Otherwise, returns the trimmed string (for URLs)
pub fn normalize_url_or_path(input: &str) -> String {
    let trimmed = input.trim();
    // Local detection: absolute/relative/Windows style or existing path
    if is_probably_local_path(trimmed) {
        let path = normalize_user_input_path(trimmed);
        return path.to_string_lossy().to_string();
    }
    trimmed.to_string()
}

fn is_probably_local_path(path: &str) -> bool {
    // Absolute POSIX or Windows drive letter
    if path.starts_with('/') || (path.len() >= 3 && path.chars().nth(1) == Some(':')) {
        return true;
    }
    // Home expansion prefix
    if path.starts_with('~') {
        return true;
    }
    // Relative prefixes
    if path.starts_with("./") || path.starts_with("../") {
        return true;
    }
    // As a last resort, check existence (avoids false positives for URLs)
    std::path::Path::new(path).exists()
}


