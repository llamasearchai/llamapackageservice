use crate::error::Result;
use env_logger::{Builder, Env};
use log::{self, LevelFilter};
use chrono::Local;
use std::io::Write;
use yansi::Paint;

/// Initializes the application's logging system with the specified log level
///
/// Valid log levels are: error, warn, info, debug, trace
pub fn init(log_level: &str) -> Result<()> {
    let env = Env::default()
        .filter_or("RUST_LOG", log_level)
        .write_style_or("RUST_LOG_STYLE", "always");

    Builder::from_env(env)
        .format(|buf, record| {
            writeln!(buf, "{}", format_log(record))
        })
        .init();

    Ok(())
}

/// Formats a log record into a structured string
///
/// Returns a formatted string with timestamp, level, and message
pub fn format_log(record: &log::Record) -> String {
    let level = match record.level() {
        log::Level::Error => Paint::red("ERROR").bold(),
        log::Level::Warn => Paint::yellow("WARN ").bold(),
        log::Level::Info => Paint::cyan("INFO ").bold(),
        log::Level::Debug => Paint::blue("DEBUG").bold(),
        log::Level::Trace => Paint::new("TRACE"),
    };

    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let target = if !record.target().is_empty() {
        record.target()
    } else {
        record.module_path().unwrap_or("unknown")
    };

    format!(
        "[{}] {} [{}] {}",
        timestamp,
        level,
        target,
        record.args()
    )
}

/// Parses a log level string into a LevelFilter
///
/// Returns the corresponding LevelFilter, defaulting to Info for invalid strings
pub fn parse_log_level(level: &str) -> LevelFilter {
    match level.to_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn, 
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Info, // Default fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_level() {
        assert_eq!(parse_log_level("error"), LevelFilter::Error);
        assert_eq!(parse_log_level("warn"), LevelFilter::Warn);
        assert_eq!(parse_log_level("info"), LevelFilter::Info);
        assert_eq!(parse_log_level("debug"), LevelFilter::Debug);
        assert_eq!(parse_log_level("trace"), LevelFilter::Trace);
        assert_eq!(parse_log_level("invalid"), LevelFilter::Info);
    }
} 