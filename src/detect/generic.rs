// Generic/fallback language detector
//
// Applied when no specific language is detected. Provides minimal
// context: file counts, build file detection, etc.

use std::path::PathBuf;
use super::LanguageDetector;

/// Fallback detector for unrecognized project types.
///
/// Always applies (the `detect` method always returns `true`),
/// making it suitable as a last-resort detector.
pub struct GenericDetector;

impl LanguageDetector for GenericDetector {
    fn name(&self) -> &'static str {
        "Generic"
    }

    fn detect(&self, _files: &[PathBuf]) -> bool {
        true // Always applies as a fallback
    }
}
