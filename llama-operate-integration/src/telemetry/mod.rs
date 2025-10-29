/// Telemetry and observability module
use anyhow::Result;
use tracing::{info, warn, error, debug, span, Level};
use tracing_subscriber::{
    fmt,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};

pub fn init_telemetry() -> Result<()> {
    // Set up tracing subscriber with multiple layers
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    let fmt_layer = fmt::layer()
        .with_target(true)
        .with_thread_ids(true)
        .with_level(true)
        .pretty();
    
    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();
    
    info!("Telemetry initialized");
    
    Ok(())
}

/// Create a span for tracking operations
#[macro_export]
macro_rules! operation_span {
    ($name:expr) => {
        tracing::info_span!("operation", name = $name)
    };
    ($name:expr, $($field:tt)*) => {
        tracing::info_span!("operation", name = $name, $($field)*)
    };
}

/// Log operation metrics
#[macro_export]
macro_rules! log_metrics {
    ($($key:expr => $value:expr),*) => {
        tracing::info!(
            target: "metrics",
            $($key = $value,)*
        );
    };
}