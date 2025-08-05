use anyhow::{anyhow, Context, Result};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_logging() -> Result<WorkerGuard> {
    let fmt_layer = tracing_subscriber::fmt::layer().with_target(true);
    // .with_thread_ids(true)
    // .with_thread_names(true)
    // .pretty();
    let file_appender = tracing_appender::rolling::Builder::new()
        .filename_prefix("annotate")
        .filename_suffix("log")
        .rotation(tracing_appender::rolling::Rotation::NEVER)
        // .max_log_files(1)
        .build(get_log_directory().context("failed to get log directory")?)
        .context("Failed to build the rolling log file appender.")?;
    let (non_blocking, appender_guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_target(true);

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::from_env("DRAW_LOG"))
        .with(fmt_layer)
        .with(file_layer)
        .init();
    Ok(appender_guard)
}
pub fn get_log_directory() -> Result<std::path::PathBuf> {
    if cfg!(target_os = "linux") {
        Ok(dirs::state_dir()
            .context("failed to get linux state dir, please set $XDG_STATE_HOME or $HOME")?
            .to_owned())
    } else if cfg!(target_os = "macos") {
        Ok(dirs::home_dir()
            .context("failed to get macos home dir")?
            .join("Library/Logs/"))
    } else if cfg!(target_os = "windows") {
        Ok(dirs::data_dir()
            .context("failed to get windows data dir")?
            .to_owned())
    } else {
        Err(anyhow!(
            "unsupported platform, if you wish for this support to be added, please make an issue on the project page."
        ))
    }
}
