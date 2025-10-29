/// Cache utilities for string and memory caching
pub mod cache;
/// Retry utilities for handling transient failures
pub mod retry;
/// Path normalization utilities for handling user input
pub mod path;
/// Permission elevation utilities for accessing restricted paths
pub mod permissions;

pub use crate::cache::Cache;
pub use retry::with_retry; 
pub use path::{normalize_user_input_path, normalize_url_or_path};
pub use permissions::{attempt_permission_elevation, has_elevated_privileges, show_elevation_hint};