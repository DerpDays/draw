use thiserror::Error;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Error, Debug)]
pub enum LoggingInitError {
    #[error("failed to get the log directory, please set $XDG_RUNTIME_DIR or $HOME")]
    UnableToGetLogDir(#[from] crate::dirs::DirectoryError),
    #[error("failed to build ")]
    Disconnect(#[from] tracing_appender::rolling::InitError),
}

pub fn init_logging() -> Result<WorkerGuard, LoggingInitError> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
    // .with_thread_ids(true)
    // .with_thread_names(true)
    // .pretty();

    let file_appender = tracing_appender::rolling::Builder::new()
        .filename_prefix("annotate")
        .filename_suffix("log")
        .rotation(tracing_appender::rolling::Rotation::NEVER)
        // .max_log_files(1)
        .build(crate::dirs::log_dir()?)?;

    let (non_blocking, appender_guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(true);

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::from_env(
            crate::APP_LOG_ENV,
        ))
        .with(fmt_layer)
        .with(file_layer)
        .init();
    Ok(appender_guard)
}
