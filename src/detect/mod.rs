// Language detection framework
//
// This module defines the LanguageDetector trait and common types
// used by all language-specific detectors (Python, JavaScript,
// Rust, Go, etc.). Each detector implements the trait and registers
// itself via the all_detectors() function.

pub mod python;
pub mod javascript;
pub mod rust;
pub mod golang;
pub mod generic;

use std::path::PathBuf;

use crate::error::CtxpkgError;

// ── Re-exports ──────────────────────────────────────────────────────────────

pub use python::PythonDetector;
pub use javascript::JavaScriptDetector;
pub use rust::RustDetector;
pub use golang::GoDetector;
pub use generic::GenericDetector;

// ── Data Types ──────────────────────────────────────────────────────────────

/// A symbol in the project's API surface.
///
/// Examples: exported functions, classes, structs, traits, HTTP routes.
#[derive(Debug, Clone)]
pub struct ApiSymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub file: PathBuf,
    pub line: usize,
    pub visibility: Visibility,
}

/// The kind of an API symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Struct,
    Class,
    Trait,
    Enum,
    Type,
    Constant,
    Route,
}

/// Visibility of an API symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Visibility {
    /// Visible outside the module (e.g., `pub` in Rust, `export` in JS/TS)
    Public,
    /// Visible outside the package but not necessarily part of the public API
    Exported,
    /// Internal/private to the module
    Private,
}

/// A dependency discovered in the project.
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub purpose: Option<String>,
}

/// A configuration file with a summary of its key content.
#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub content_summary: String,
}

// ── LanguageDetector Trait ──────────────────────────────────────────────────

/// Common interface all language detectors must implement.
///
/// Each detector knows how to:
/// - Identify whether it applies to a given set of project files
/// - Extract API surface symbols (functions, types, classes, routes)
/// - Extract dependency information
/// - Extract configuration file summaries
pub trait LanguageDetector {
    /// Human-readable name of this detector (e.g., "Rust", "Python").
    fn name(&self) -> &'static str;

    /// Return `true` if this detector applies to the given set of files.
    ///
    /// Typically checks for the presence of specific build/config files
    /// (e.g., `Cargo.toml` for Rust, `package.json` for JavaScript).
    fn detect(&self, files: &[PathBuf]) -> bool;

    /// Extract API surface symbols from the project files.
    ///
    /// Returns a list of `ApiSymbol` entries representing the public API.
    fn extract_api_surface(&self, _files: &[PathBuf]) -> Result<Vec<ApiSymbol>, CtxpkgError> {
        Ok(Vec::new())
    }

    /// Extract dependencies from the project files.
    ///
    /// Returns a list of `Dependency` entries with name, version, and purpose.
    fn extract_deps(&self, _files: &[PathBuf]) -> Result<Vec<Dependency>, CtxpkgError> {
        Ok(Vec::new())
    }

    /// Extract configuration files with content summaries.
    ///
    /// Returns a list of `ConfigFile` entries for build, CI, linter, etc.
    fn extract_configs(&self, _files: &[PathBuf]) -> Result<Vec<ConfigFile>, CtxpkgError> {
        Ok(Vec::new())
    }
}

// ── Detector Registry ───────────────────────────────────────────────────────

/// Return all available language detectors in order of specificity.
///
/// More specific detectors (Rust, Go) are checked before generic ones.
pub fn all_detectors() -> Vec<Box<dyn LanguageDetector>> {
    vec![
        Box::new(RustDetector),
        Box::new(GoDetector),
        Box::new(PythonDetector),
        Box::new(JavaScriptDetector),
        Box::new(GenericDetector),
    ]
}

/// Aggregate context from all applicable language detectors.
///
/// This runs detection against all available detectors and merges
/// their API surface, dependency, and config information into a
/// single `LanguageContext` result.
pub fn detect_and_extract(
    root: &std::path::Path,
    files: &[PathBuf],
) -> Result<LanguageContext, CtxpkgError> {
    let mut api_surface = Vec::new();
    let mut dependencies = Vec::new();
    let mut config_files = Vec::new();
    let mut detected_languages = Vec::new();

    for detector in all_detectors() {
        if detector.detect(files) {
            detected_languages.push(detector.name().to_string());

            if let Ok(symbols) = detector.extract_api_surface(files) {
                api_surface.extend(symbols);
            }
            if let Ok(deps) = detector.extract_deps(files) {
                dependencies.extend(deps);
            }
            if let Ok(cfgs) = detector.extract_configs(files) {
                config_files.extend(cfgs);
            }
        }
    }

    Ok(LanguageContext {
        root: root.to_path_buf(),
        detected_languages,
        api_surface,
        dependencies,
        config_files,
    })
}

/// The aggregated result of running all applicable language detectors.
#[derive(Debug, Clone)]
pub struct LanguageContext {
    /// Project root path
    pub root: PathBuf,
    /// Names of all detected languages (e.g., ["Rust", "JavaScript"])
    pub detected_languages: Vec<String>,
    /// Combined API surface from all detectors
    pub api_surface: Vec<ApiSymbol>,
    /// Combined dependencies from all detectors
    pub dependencies: Vec<Dependency>,
    /// Combined config files from all detectors
    pub config_files: Vec<ConfigFile>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_files(files: &[&str]) -> Vec<PathBuf> {
        let dir = std::env::temp_dir().join(format!("ctxpkg_detect_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut paths = Vec::new();
        for f in files {
            let path = dir.join(f);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(&path, b"").unwrap();
            paths.push(path);
        }
        paths
    }

    #[test]
    fn test_rust_detector_detects_cargo_toml() {
        let files = create_files(&["Cargo.toml", "src/main.rs"]);
        let detector = RustDetector;
        assert!(detector.detect(&files));
    }

    #[test]
    fn test_rust_detector_rejects_non_rust() {
        let files = create_files(&["package.json"]);
        let detector = RustDetector;
        assert!(!detector.detect(&files));
    }

    #[test]
    fn test_python_detector_detects_pyproject_toml() {
        let files = create_files(&["pyproject.toml"]);
        let detector = PythonDetector;
        assert!(detector.detect(&files));
    }

    #[test]
    fn test_python_detector_detects_setup_py() {
        let files = create_files(&["setup.py"]);
        let detector = PythonDetector;
        assert!(detector.detect(&files));
    }

    #[test]
    fn test_javascript_detector_detects_package_json() {
        let files = create_files(&["package.json"]);
        let detector = JavaScriptDetector;
        assert!(detector.detect(&files));
    }

    #[test]
    fn test_go_detector_detects_go_mod() {
        let files = create_files(&["go.mod"]);
        let detector = GoDetector;
        assert!(detector.detect(&files));
    }

    #[test]
    fn test_generic_detector_always_true() {
        let files = create_files(&["random_file.txt"]);
        let detector = GenericDetector;
        assert!(detector.detect(&files));
    }

    #[test]
    fn test_all_detectors_contains_all() {
        let detectors = all_detectors();
        let names: Vec<&str> = detectors.iter().map(|d| d.name()).collect();
        assert!(names.contains(&"Rust"));
        assert!(names.contains(&"Go"));
        assert!(names.contains(&"Python"));
        assert!(names.contains(&"JavaScript/TypeScript"));
        assert!(names.contains(&"Generic"));
    }

    #[test]
    fn test_python_api_surface_extracts_defs() {
        let dir = std::env::temp_dir().join(format!("ctxpkg_py_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let py_file = dir.join("my_module.py");
        std::fs::write(
            &py_file,
            b"def hello():\n    pass\n\nclass MyClass:\n    pass\n\nasync def async_fn():\n    pass\n",
        )
        .unwrap();

        let detector = PythonDetector;
        let symbols = detector.extract_api_surface(&[py_file]).unwrap();
        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"hello"));
        assert!(names.contains(&"MyClass"));
        assert!(names.contains(&"async_fn"));
    }

    #[test]
    fn test_detect_and_extract_with_multiple_detectors() {
        let files = create_files(&["Cargo.toml", "package.json", "src/main.rs", "index.ts"]);
        let root = files[0].parent().unwrap();
        let ctx = detect_and_extract(root, &files).unwrap();
        assert!(ctx.detected_languages.contains(&"Rust".to_string()));
        assert!(
            ctx.detected_languages
                .contains(&"JavaScript/TypeScript".to_string())
        );
    }

    #[test]
    fn test_empty_project_generic_only() {
        let files = create_files(&["README.md", "LICENSE"]);
        let root = files[0].parent().unwrap();
        let ctx = detect_and_extract(root, &files).unwrap();
        // Generic always applies
        assert!(ctx.detected_languages.contains(&"Generic".to_string()));
    }
}
