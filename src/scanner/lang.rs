// Language detection framework — heuristic-based detection
// Full implementation comes in Phase 2.5

use std::path::PathBuf;

/// A detected language with its metadata
#[derive(Debug, Clone)]
pub struct DetectedLanguage {
    pub name: String,
    pub version: Option<String>,
    pub build_files: Vec<PathBuf>,
    pub detector_used: String,
}

/// Detect the primary language of a project based on build/config files.
/// Returns a DetectedLanguage if one can be identified, otherwise None.
pub fn detect_language(root: &std::path::Path) -> Option<DetectedLanguage> {
    // Heuristic checks for known build files
    if root.join("Cargo.toml").exists() {
        return Some(DetectedLanguage {
            name: "Rust".into(),
            version: None,
            build_files: vec![root.join("Cargo.toml")],
            detector_used: "build-file".into(),
        });
    }
    if root.join("package.json").exists() {
        return Some(DetectedLanguage {
            name: "JavaScript/TypeScript".into(),
            version: None,
            build_files: vec![root.join("package.json")],
            detector_used: "build-file".into(),
        });
    }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        return Some(DetectedLanguage {
            name: "Python".into(),
            version: None,
            build_files: vec![root.join("pyproject.toml")],
            detector_used: "build-file".into(),
        });
    }
    if root.join("go.mod").exists() {
        return Some(DetectedLanguage {
            name: "Go".into(),
            version: None,
            build_files: vec![root.join("go.mod")],
            detector_used: "build-file".into(),
        });
    }
    if root.join("Gemfile").exists() {
        return Some(DetectedLanguage {
            name: "Ruby".into(),
            version: None,
            build_files: vec![root.join("Gemfile")],
            detector_used: "build-file".into(),
        });
    }
    None
}
