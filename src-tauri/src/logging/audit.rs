use std::path::Path;
use tracing_appender;
use tracing_subscriber::{fmt, EnvFilter};

/// Initialize the tracing subscriber for structured logging.
/// Logs to both stderr and a file in the given log directory.
pub fn init_logging_to(log_dir: &Path) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shelldeck_lib=debug,russh=warn"));

    let file_appender = tracing_appender::rolling::daily(log_dir, "shelldeck.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // Leak the guard so the writer stays alive for the process lifetime
    std::mem::forget(_guard);

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();
}

/// Fallback: init logging to stderr only (used before data dir is known).
pub fn init_logging() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("shelldeck_lib=info,russh=warn"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(false)
        .with_file(true)
        .with_line_number(true)
        .init();
}
