// JavaScript/TypeScript language detector
//
// Detection: package.json
// API surface: exported functions, classes, route handlers (to be expanded)

use std::path::PathBuf;
use super::LanguageDetector;

/// Detector for JavaScript/TypeScript projects.
pub struct JavaScriptDetector;

impl LanguageDetector for JavaScriptDetector {
    fn name(&self) -> &'static str {
        "JavaScript/TypeScript"
    }

    fn detect(&self, files: &[PathBuf]) -> bool {
        files.iter().any(|f| {
            f.file_name().and_then(|n| n.to_str()) == Some("package.json")
        })
    }
}
