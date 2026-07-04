use thiserror::Error;

#[derive(Error, Debug)]
pub enum CtxpkgError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Scan error: {0}")]
    ScanError(String),

    #[error("Language detection error: {0}")]
    DetectError(String),

    #[error("Pack error: {0}")]
    PackError(String),

    #[error("Clipboard error: {0}")]
    ClipboardError(String),
}

impl From<std::io::Error> for CtxpkgError {
    fn from(e: std::io::Error) -> Self {
        CtxpkgError::ScanError(e.to_string())
    }
}

impl From<serde_yaml::Error> for CtxpkgError {
    fn from(e: serde_yaml::Error) -> Self {
        CtxpkgError::ConfigError(e.to_string())
    }
}
