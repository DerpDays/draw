use std::path::PathBuf;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum DirectoryError {
    #[error("failed to get the log directory, please set $XDG_RUNTIME_DIR or $HOME")]
    UnableToGetLogDir,
    #[error("failed to get the config directory, please set $XDG_CONFIG_DIR or $HOME")]
    UnableToGetConfigDir,
    #[error("failed to get the runtime directory, please set $XDG_RUNTIME_DIR")]
    UnableToGetRuntimeDir,
}

pub fn log_dir() -> Result<PathBuf, DirectoryError> {
    std::env::var("XDG_STATE_DIR").map_or_else(
        |_| {
            std::env::home_dir().map_or(Err(DirectoryError::UnableToGetLogDir), |mut path| {
                path.push(".local");
                path.push(".state");
                Ok(path)
            })
        },
        |dir| Ok(PathBuf::from(dir)),
    )
}

pub fn runtime_dir() -> Result<PathBuf, DirectoryError> {
    std::env::var("XDG_RUNTIME_DIR").map_or(Err(DirectoryError::UnableToGetRuntimeDir), |dir| {
        Ok(PathBuf::from(dir))
    })
}

pub fn config_dir() -> Result<PathBuf, DirectoryError> {
    std::env::var("XDG_CONFIG_DIR").map_or_else(
        |_| {
            std::env::home_dir().map_or(Err(DirectoryError::UnableToGetConfigDir), |mut path| {
                path.push(".config");
                path.push(crate::APP_NAME);
                Ok(path)
            })
        },
        |dir| {
            let mut path = PathBuf::from(dir);
            path.push(crate::APP_NAME);
            Ok(path)
        },
    )
}
