// Go language detector
//
// Detection: go.mod
// API surface: exported functions/types (capital-letter naming)

use std::path::PathBuf;
use super::LanguageDetector;

/// Detector for Go projects.
pub struct GoDetector;

impl LanguageDetector for GoDetector {
    fn name(&self) -> &'static str {
        "Go"
    }

    fn detect(&self, files: &[PathBuf]) -> bool {
        files.iter().any(|f| {
            f.file_name().and_then(|n| n.to_str()) == Some("go.mod")
        })
    }
}
