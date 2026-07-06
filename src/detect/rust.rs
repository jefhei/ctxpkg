// Rust language detector
//
// Detection: Cargo.toml
// API surface: pub fn, pub struct, pub enum, pub trait, pub mod, pub type

use std::path::PathBuf;
use super::LanguageDetector;

/// Detector for Rust projects.
pub struct RustDetector;

impl LanguageDetector for RustDetector {
    fn name(&self) -> &'static str {
        "Rust"
    }

    fn detect(&self, files: &[PathBuf]) -> bool {
        files.iter().any(|f| {
            f.file_name().and_then(|n| n.to_str()) == Some("Cargo.toml")
        })
    }
}
