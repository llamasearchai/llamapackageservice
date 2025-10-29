use std::path::PathBuf;

/// Normalize user-provided file paths by trimming whitespace, removing quotes, and expanding tilde
///
/// This function handles common path input formats:
/// - Trailing/leading whitespace
/// - Single or double quotes surrounding the path
/// - Tilde (~) expansion to home directory
///
/// # Arguments
/// * `input` - The raw path string from user input
///
/// # Returns
/// A normalized `PathBuf` ready for file system operations
///
/// # Examples
/// ```
/// use llamapackageservice::utils::normalize_user_input_path;
///
/// let path = normalize_user_input_path("/tmp/test ");
/// assert_eq!(path.to_string_lossy(), "/tmp/test");
///
/// let path = normalize_user_input_path("\"~/Documents\"");
/// // Returns: /home/user/Documents (or equivalent on your system)
/// ```
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
        let without_tilde = unquoted
            .strip_prefix("~/")
            .or_else(|| unquoted.strip_prefix("~"))
            .unwrap_or(unquoted);
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

/// Detect if a string is likely a local path and normalize it, otherwise return as-is
///
/// This function distinguishes between URLs (http/https) and local file paths,
/// applying path normalization only to local paths.
///
/// # Arguments
/// * `input` - The raw input string (URL or path)
///
/// # Returns
/// A normalized string, either a normalized path or the original URL
///
/// # Examples
/// ```
/// use llamapackageservice::utils::normalize_url_or_path;
///
/// let url = normalize_url_or_path("https://github.com/user/repo");
/// assert_eq!(url, "https://github.com/user/repo");
///
/// let path = normalize_url_or_path("/tmp/project ");
/// assert_eq!(path, "/tmp/project");
/// ```
pub fn normalize_url_or_path(input: &str) -> String {
    let trimmed = input.trim();
    
    // Check if it's likely a local path
    if is_probably_local_path(trimmed) {
        let path = normalize_user_input_path(trimmed);
        return path.to_string_lossy().to_string();
    }
    
    trimmed.to_string()
}

/// Heuristic to determine if a string is likely a local file path rather than a URL
fn is_probably_local_path(s: &str) -> bool {
    // Absolute paths
    if s.starts_with('/') || s.starts_with('\\') {
        return true;
    }
    
    // Home directory paths
    if s.starts_with('~') {
        return true;
    }
    
    // Relative paths
    if s.starts_with("./") || s.starts_with("../") {
        return true;
    }
    
    // Windows paths
    if s.len() >= 3 && s.chars().nth(1) == Some(':') {
        return true;
    }
    
    // If it starts with a URL scheme, it's not a local path
    if s.starts_with("http://") || s.starts_with("https://") || s.starts_with("file://") {
        return false;
    }
    
    // If it contains a dot followed by a slash early on, assume it's a path
    if s.starts_with("./") || s.starts_with("../") {
        return true;
    }
    
    // Default: if it doesn't look like a URL, treat as path
    !s.contains("://")
}
