# Documentation Guidelines

## Module Documentation Structure

Each module should follow this documentation structure:

1. **Module-Level Documentation** (at the top of the file):
```rust
//! # Module Name
//! 
//! Brief description of the module's purpose.
//!
//! ## Features
//! - Feature 1
//! - Feature 2
//!
//! ## Usage
//! ```rust
//! // Example code
//! ```
```

2. **Public Item Documentation**:
```rust
/// Describes what this function does
///
/// # Arguments
/// * `arg1` - Description of first argument
/// * `arg2` - Description of second argument
///
/// # Returns
/// Description of return value
///
/// # Examples
/// ```rust
/// // Example usage
/// ```
pub fn my_function(arg1: Type1, arg2: Type2) -> ReturnType {
    // Implementation
}
```

## Best Practices

1. Place module documentation (`//!`) only at the top of the file
2. Use regular documentation (`///`) for items (functions, structs, etc.)
3. Include examples in documentation
4. Document error conditions and handling
5. Keep documentation up to date with code changes 