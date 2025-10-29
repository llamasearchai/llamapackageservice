use std::path::Path;
use std::process::Command;
use log::{warn, info};

/// Attempt to elevate permissions for accessing a restricted path
///
/// This function checks if a path requires elevated permissions and attempts
/// to gain access using platform-specific methods (sudo on Unix-like systems).
///
/// # Arguments
/// * `path` - The path that requires elevated permissions
///
/// # Returns
/// * `Ok(true)` if permissions were successfully elevated
/// * `Ok(false)` if elevation was not needed or not possible
/// * `Err` if there was an error checking permissions
pub fn attempt_permission_elevation(path: &Path) -> std::io::Result<bool> {
    // Check if we can already access the path
    if path.exists() && path.metadata().is_ok() {
        return Ok(false); // No elevation needed
    }

    // Check if path requires root/elevated permissions
    if is_root_path(path) {
        info!("Path requires elevated permissions: {}", path.display());
        
        #[cfg(target_family = "unix")]
        {
            return elevate_unix_permissions(path);
        }
        
        #[cfg(target_family = "windows")]
        {
            return elevate_windows_permissions(path);
        }
        
        #[cfg(not(any(target_family = "unix", target_family = "windows")))]
        {
            warn!("Permission elevation not implemented for this platform");
            Ok(false)
        }
    } else {
        Ok(false)
    }
}

/// Check if a path typically requires root/elevated permissions
fn is_root_path(path: &Path) -> bool {
    let path_str = path.to_string_lossy();
    
    // Common root-only paths on Unix-like systems
    #[cfg(target_family = "unix")]
    {
        if path_str.starts_with("/etc/") 
            || path_str.starts_with("/var/log/")
            || path_str.starts_with("/root/")
            || path_str.starts_with("/sys/")
            || path_str.starts_with("/proc/") {
            return true;
        }
    }
    
    // Check if parent directories have restrictive permissions
    if let Some(parent) = path.parent() {
        if let Ok(metadata) = parent.metadata() {
            let permissions = metadata.permissions();
            // On Unix, check if we can read the parent directory
            #[cfg(target_family = "unix")]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = permissions.mode();
                // Check if owner-only read (0400) or similar restrictive permissions
                if mode & 0o444 == 0 {
                    return true;
                }
            }
        }
    }
    
    false
}

#[cfg(target_family = "unix")]
fn elevate_unix_permissions(path: &Path) -> std::io::Result<bool> {
    use std::os::unix::fs::PermissionsExt;
    
    // Check if we're already root
    let uid = unsafe { libc::getuid() };
    if uid == 0 {
        info!("Already running as root");
        return Ok(true);
    }
    
    // Try to make the path accessible by changing permissions
    // Note: This won't work for paths we don't own, but worth trying
    warn!("Attempting to access restricted path: {}", path.display());
    warn!("Note: For restricted system paths, run the program with sudo");
    
    // We can't actually elevate within the same process without sudo
    // Just log and continue - the graceful error handling will skip inaccessible files
    Ok(false)
}

#[cfg(target_family = "windows")]
fn elevate_windows_permissions(_path: &Path) -> std::io::Result<bool> {
    // On Windows, we would need to request UAC elevation
    // This typically requires restarting the process
    warn!("Permission elevation on Windows requires running as Administrator");
    Ok(false)
}

/// Check if the current process has elevated privileges
pub fn has_elevated_privileges() -> bool {
    #[cfg(target_family = "unix")]
    {
        let uid = unsafe { libc::getuid() };
        uid == 0
    }
    
    #[cfg(target_family = "windows")]
    {
        // On Windows, we'd check if running as Administrator
        // This is a simplified check
        false
    }
    
    #[cfg(not(any(target_family = "unix", target_family = "windows")))]
    {
        false
    }
}

/// Display a helpful message about running with elevated permissions
pub fn show_elevation_hint(path: &Path) {
    if is_root_path(path) && !has_elevated_privileges() {
        warn!("To process restricted paths like '{}', consider running with:", path.display());
        
        #[cfg(target_family = "unix")]
        warn!("  sudo llamapackageservice --url \"{}\"", path.display());
        
        #[cfg(target_family = "windows")]
        warn!("  Run as Administrator");
        
        warn!("Otherwise, accessible files will be processed and restricted files will be skipped.");
    }
}

