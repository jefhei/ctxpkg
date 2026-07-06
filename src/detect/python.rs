// Python language detector
//
// Detection: pyproject.toml, setup.py, requirements.txt, Pipfile
// API surface: top-level functions, classes, async defs

use std::path::PathBuf;
use crate::error::CtxpkgError;
use super::{ApiSymbol, LanguageDetector, SymbolKind, Visibility};

/// Detector for Python projects.
pub struct PythonDetector;

impl LanguageDetector for PythonDetector {
    fn name(&self) -> &'static str {
        "Python"
    }

    fn detect(&self, files: &[PathBuf]) -> bool {
        files.iter().any(|f| {
            let name = f.file_name().and_then(|n| n.to_str()).unwrap_or("");
            name == "pyproject.toml"
                || name == "setup.py"
                || name == "requirements.txt"
                || name == "Pipfile"
        })
    }

    fn extract_api_surface(&self, files: &[PathBuf]) -> Result<Vec<ApiSymbol>, CtxpkgError> {
        let mut symbols = Vec::new();
        for path in files {
            if path.extension().and_then(|e| e.to_str()) != Some("py") {
                continue;
            }
            let content = std::fs::read_to_string(path).map_err(|e| {
                CtxpkgError::DetectError(format!("Cannot read {}: {}", path.display(), e))
            })?;
            for (i, line) in content.lines().enumerate() {
                let trimmed = line.trim();
                if trimmed.starts_with("def ") {
                    let name = trimmed
                        .strip_prefix("def ")
                        .and_then(|s| s.split(|c: char| !c.is_alphanumeric() && c != '_').next())
                        .unwrap_or("<anonymous>")
                        .to_string();
                    symbols.push(ApiSymbol {
                        name,
                        kind: SymbolKind::Function,
                        file: path.to_path_buf(),
                        line: i + 1,
                        visibility: Visibility::Public,
                    });
                } else if trimmed.starts_with("async def ") {
                    let name = trimmed
                        .strip_prefix("async def ")
                        .and_then(|s| s.split(|c: char| !c.is_alphanumeric() && c != '_').next())
                        .unwrap_or("<anonymous>")
                        .to_string();
                    symbols.push(ApiSymbol {
                        name,
                        kind: SymbolKind::Function,
                        file: path.to_path_buf(),
                        line: i + 1,
                        visibility: Visibility::Public,
                    });
                } else if trimmed.starts_with("class ") {
                    let name = trimmed
                        .strip_prefix("class ")
                        .and_then(|s| s.split(|c: char| c == ':' || c == '(' || c.is_whitespace()).next())
                        .unwrap_or("<anonymous>")
                        .to_string();
                    symbols.push(ApiSymbol {
                        name,
                        kind: SymbolKind::Class,
                        file: path.to_path_buf(),
                        line: i + 1,
                        visibility: Visibility::Public,
                    });
                }
            }
        }
        Ok(symbols)
    }
}
