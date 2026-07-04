// Scanner — project directory walking and structure discovery

pub mod git;
pub mod lang;

use std::path::{Path, PathBuf};
use std::collections::HashSet;

use crate::config::Config;
use crate::error::CtxpkgError;

/// The scanner walks a project directory and produces a ScanResult.
pub struct Scanner {
    root: PathBuf,
    config: Config,
}

impl Scanner {
    /// Create a new scanner for the given project root and config.
    pub fn new(root: PathBuf, config: Config) -> Self {
        Self { root, config }
    }

    /// Scan the project directory and produce a structured result.
    pub fn scan(&self) -> Result<ScanResult, CtxpkgError> {
        let tree = DirectoryTree::build(
            &self.root,
            self.config.pack.max_depth,
            &self.config.pack.exclude_patterns,
        )?;

        let file_count = tree.file_count();
        let source_files = collect_source_files(&self.root);
        let config_files = collect_config_files(&self.root);

        let git_info = git::get_git_info(&self.root);
        let detected_language = lang::detect_language(&self.root)
            .map(|dl| dl.name);

        // Simple build system detection based on known files
        let build_system = detect_build_system(&self.root);

        Ok(ScanResult {
            root: self.root.clone(),
            tree,
            file_count,
            detected_language,
            build_system,
            config_files,
            source_files,
            git_info,
        })
    }
}

/// The result of scanning a project directory.
#[derive(Debug, Clone)]
pub struct ScanResult {
    pub root: PathBuf,
    pub tree: DirectoryTree,
    pub file_count: usize,
    pub detected_language: Option<String>,
    pub build_system: Option<String>,
    pub config_files: Vec<PathBuf>,
    pub source_files: Vec<PathBuf>,
    pub git_info: Option<git::GitInfo>,
}

/// A node in the project's directory tree.
#[derive(Debug, Clone)]
pub enum TreeNode {
    File {
        name: String,
        path: PathBuf,
        size: u64,
    },
    Dir {
        name: String,
        children: Vec<TreeNode>,
        file_count: usize,
    },
}

impl TreeNode {
    /// Count files recursively under this node.
    pub fn file_count(&self) -> usize {
        match self {
            TreeNode::File { .. } => 1,
            TreeNode::Dir { file_count, .. } => *file_count,
        }
    }
}

/// A condensed directory tree representation.
#[derive(Debug, Clone)]
pub struct DirectoryTree {
    pub root: TreeNode,
}

impl DirectoryTree {
    /// Build a directory tree from the given root path, with depth cap and exclude patterns.
    pub fn build(
        root: &Path,
        max_depth: usize,
        exclude_patterns: &[String],
    ) -> Result<Self, CtxpkgError> {
        // Convert exclude patterns into a set of directory/file names to skip
        let skip_dirs: HashSet<&str> = [
            "node_modules",
            "target",
            ".git",
            "vendor",
            "__pycache__",
            ".ctxpkg",
        ]
        .into_iter()
        .collect();

        let root_node = build_tree_recursive(root, root, max_depth, 0, &skip_dirs, exclude_patterns)?;
        Ok(DirectoryTree { root: root_node })
    }

    /// Total file count across the entire tree.
    pub fn file_count(&self) -> usize {
        self.root.file_count()
    }
}

/// Recursively build tree nodes using std::fs walking (respects depth and skip lists).
fn build_tree_recursive(
    abs_root: &Path,
    current: &Path,
    max_depth: usize,
    depth: usize,
    skip_dirs: &HashSet<&str>,
    exclude_patterns: &[String],
) -> Result<TreeNode, CtxpkgError> {
    let name = current
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| current.to_string_lossy().to_string());

    if !current.is_dir() {
        let metadata = std::fs::metadata(current)
            .map_err(|e| CtxpkgError::ScanError(format!("Failed to read metadata for {}: {}", current.display(), e)))?;
        return Ok(TreeNode::File {
            name,
            path: current.to_path_buf(),
            size: metadata.len(),
        });
    }

    // Check if this directory should be skipped
    if let Some(dir_name) = current.file_name().and_then(|n| n.to_str()) {
        if skip_dirs.contains(dir_name) {
            return Ok(TreeNode::Dir {
                name,
                children: vec![],
                file_count: 0,
            });
        }
    }

    // Read directory entries
    let entries = std::fs::read_dir(current)
        .map_err(|e| CtxpkgError::ScanError(format!("Failed to read directory {}: {}", current.display(), e)))?;

    let mut children = Vec::new();
    let mut total_files = 0;

    for entry in entries {
        let entry = entry
            .map_err(|e| CtxpkgError::ScanError(format!("Failed to read entry: {}", e)))?;
        let path = entry.path();

        // Skip entries matching exclude patterns (simple filename matching)
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let relative = path
            .strip_prefix(abs_root)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        if should_exclude(file_name, &relative, exclude_patterns) {
            continue;
        }

        if path.is_dir() && depth >= max_depth {
            // Depth limit reached — add as a collapsed directory
            let sub_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            // Count files in this subdirectory without traversing deeper
            let sub_count = count_files_in_dir(&path, skip_dirs)?;
            if sub_count > 0 || !is_skipped_dir(&path, skip_dirs) {
                children.push(TreeNode::Dir {
                    name: sub_name,
                    children: vec![],
                    file_count: sub_count,
                });
                total_files += sub_count;
            }
        } else if path.is_dir() {
            let child = build_tree_recursive(
                abs_root,
                &path,
                max_depth,
                depth + 1,
                skip_dirs,
                exclude_patterns,
            )?;
            let fc = child.file_count();
            if fc > 0 || !is_skipped_dir(&path, skip_dirs) {
                total_files += fc;
                children.push(child);
            }
        } else if path.is_file() {
            let metadata = std::fs::metadata(&path)
                .map_err(|e| CtxpkgError::ScanError(format!("Failed to read metadata: {}", e)))?;
            children.push(TreeNode::File {
                name: file_name.to_string(),
                path: path.clone(),
                size: metadata.len(),
            });
            total_files += 1;
        }
    }

    // Sort children: directories first, then alphabetically
    children.sort_by(|a, b| {
        let a_is_dir = matches!(a, TreeNode::Dir { .. });
        let b_is_dir = matches!(b, TreeNode::Dir { .. });
        if a_is_dir != b_is_dir {
            b_is_dir.cmp(&a_is_dir)
        } else {
            let a_name = match a {
                TreeNode::File { name, .. } => name.as_str(),
                TreeNode::Dir { name, .. } => name.as_str(),
            };
            let b_name = match b {
                TreeNode::File { name, .. } => name.as_str(),
                TreeNode::Dir { name, .. } => name.as_str(),
            };
            a_name.cmp(b_name)
        }
    });

    Ok(TreeNode::Dir {
        name,
        children,
        file_count: total_files,
    })
}

/// Check whether a file or directory name matches exclude patterns.
fn should_exclude(file_name: &str, relative_path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if pattern == file_name || pattern == relative_path {
            return true;
        }
        // Use globset for proper glob matching (e.g., *.min.*, *.map)
        if let Ok(glob) = globset::Glob::new(pattern) {
            let matcher = glob.compile_matcher();
            if matcher.is_match(file_name) || matcher.is_match(relative_path) {
                return true;
            }
        }
    }
    false
}

/// Check if a path is one of the well-known skip directories.
fn is_skipped_dir(path: &Path, skip_dirs: &HashSet<&str>) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| skip_dirs.contains(n))
        .unwrap_or(false)
}

/// Count the number of files in a directory (non-recursive level only).
fn count_files_in_dir(dir: &Path, skip_dirs: &HashSet<&str>) -> Result<usize, CtxpkgError> {
    let mut count = 0;
    if let Some(dir_name) = dir.file_name().and_then(|n| n.to_str()) {
        if skip_dirs.contains(dir_name) {
            return Ok(0);
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                count += 1;
            } else if path.is_dir() {
                // Recurse one level deeper for counting
                if let Ok(sub) = std::fs::read_dir(&path) {
                    count += sub.flatten().filter(|e| e.path().is_file()).count();
                }
            }
        }
    }
    Ok(count)
}

/// Collect source files (by common extensions) from a directory tree walk.
fn collect_source_files(root: &Path) -> Vec<PathBuf> {
    let source_extensions: HashSet<&str> = [
        "rs", "py", "js", "ts", "jsx", "tsx", "go", "rb",
        "java", "kt", "scala", "swift", "c", "h", "cpp", "hpp",
        "cs", "php", "r", "sh", "bash", "zsh", "fish",
    ]
    .into_iter()
    .collect();

    let skip_dirs: HashSet<&str> = [
        "node_modules", "target", ".git", "vendor", "__pycache__", ".ctxpkg",
    ]
    .into_iter()
    .collect();

    let mut files = Vec::new();
    collect_files_recursive(root, &skip_dirs, &source_extensions, &mut files);
    files
}

/// Collect config/build files from a directory tree walk.
fn collect_config_files(root: &Path) -> Vec<PathBuf> {
    let config_names: HashSet<&str> = [
        "Cargo.toml", "package.json", "pyproject.toml", "setup.py",
        "go.mod", "Gemfile", "Makefile", "Dockerfile", "docker-compose.yml",
        "Justfile", "Taskfile.yml", ".gitignore", ".env", ".env.example",
        "tsconfig.json", ".eslintrc.js", ".prettierrc", "rust-toolchain.toml",
        "Cargo.lock",
    ]
    .into_iter()
    .collect();

    let skip_dirs: HashSet<&str> = [
        "node_modules", "target", ".git", "vendor", "__pycache__", ".ctxpkg",
    ]
    .into_iter()
    .collect();

    let mut files = Vec::new();
    collect_config_recursive(root, &skip_dirs, &config_names, &mut files);
    files
}

fn collect_files_recursive(
    dir: &Path,
    skip_dirs: &HashSet<&str>,
    extensions: &HashSet<&str>,
    results: &mut Vec<PathBuf>,
) {
    if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
        if skip_dirs.contains(name) {
            return;
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, skip_dirs, extensions, results);
            } else if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if extensions.contains(ext) {
                        results.push(path);
                    }
                }
            }
        }
    }
}

fn collect_config_recursive(
    dir: &Path,
    skip_dirs: &HashSet<&str>,
    names: &HashSet<&str>,
    results: &mut Vec<PathBuf>,
) {
    if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
        if skip_dirs.contains(name) {
            return;
        }
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_config_recursive(&path, skip_dirs, names, results);
            } else if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    if names.contains(name) {
                        results.push(path);
                    }
                }
            }
        }
    }
}

/// Detect the build system from known files at the project root.
fn detect_build_system(root: &Path) -> Option<String> {
    if root.join("Cargo.toml").exists() {
        return Some("cargo".into());
    }
    if root.join("package.json").exists() {
        // Try to detect yarn/pnpm from lockfile
        if root.join("yarn.lock").exists() {
            return Some("yarn".into());
        }
        if root.join("pnpm-lock.yaml").exists() {
            return Some("pnpm".into());
        }
        return Some("npm".into());
    }
    if root.join("pyproject.toml").exists() {
        // Try to detect poetry/uv/pdm
        return Some("pip".into());
    }
    if root.join("setup.py").exists() {
        return Some("pip".into());
    }
    if root.join("go.mod").exists() {
        return Some("go".into());
    }
    if root.join("Gemfile").exists() {
        return Some("bundler".into());
    }
    if root.join("Makefile").exists() {
        return Some("make".into());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_fixture(name: &str, files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::TempDir::with_prefix(name).unwrap();
        for file in files {
            let path = dir.path().join(file);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, b"content").unwrap();
        }
        dir
    }

    #[test]
    fn test_directory_tree_builds() {
        let fixture = create_test_fixture(
            "tree_test",
            &["src/main.rs", "src/lib.rs", "README.md", "Cargo.toml"],
        );
        let tree =
            DirectoryTree::build(fixture.path(), 10, &[]).unwrap();
        assert_eq!(tree.file_count(), 4);
    }

    #[test]
    fn test_depth_limiting() {
        let fixture = create_test_fixture(
            "depth_test",
            &[
                "src/main.rs",
                "src/deep/nested/file.rs",
                "src/deep/nested/deeper/thing.rs",
            ],
        );
        // Depth 2 should not show the deepest files
        let tree =
            DirectoryTree::build(fixture.path(), 2, &[]).unwrap();
        // At depth 2, "src/deep/nested/file.rs" is depth 3, "src/deep/nested/deeper/thing.rs" is depth 4
        // src (depth 1) → deep (depth 2) → nested (depth 3, collapsed)
        // The collapsed deep dir should count its files
        assert_eq!(tree.file_count(), 3);
    }

    #[test]
    fn test_skip_directories() {
        let fixture = create_test_fixture(
            "skip_test",
            &[
                "src/main.rs",
                "node_modules/package/index.js",
                "target/debug/output",
                ".git/HEAD",
            ],
        );
        let tree =
            DirectoryTree::build(fixture.path(), 10, &[]).unwrap();
        // Only src/main.rs should be counted
        assert_eq!(tree.file_count(), 1);
    }

    #[test]
    fn test_exclude_patterns() {
        let fixture = create_test_fixture(
            "exclude_test",
            &[
                "src/main.rs",
                "dist/bundle.min.js",
                "dist/bundle.map",
            ],
        );
        let tree = DirectoryTree::build(
            fixture.path(),
            10,
            &["*.min.*".to_string(), "*.map".to_string()],
        )
        .unwrap();
        // Only src/main.rs should be counted
        assert_eq!(tree.file_count(), 1);
    }

    #[test]
    fn test_empty_directory() {
        let fixture = create_test_fixture("empty_test", &[]);
        let tree =
            DirectoryTree::build(fixture.path(), 10, &[]).unwrap();
        assert_eq!(tree.file_count(), 0);
    }

    #[test]
    fn test_scanner_scan() {
        let fixture = create_test_fixture(
            "scanner_test",
            &["src/main.rs", "src/lib.rs", "Cargo.toml", "README.md"],
        );
        let config = Config::default();
        let scanner = Scanner::new(fixture.path().to_path_buf(), config);
        let result = scanner.scan().unwrap();
        assert_eq!(result.file_count, 4);
        // Since this is a Rust project with Cargo.toml
        assert_eq!(result.detected_language.as_deref(), Some("Rust"));
        assert_eq!(result.build_system.as_deref(), Some("cargo"));
    }

    #[test]
    fn test_scanner_config_files() {
        let fixture = create_test_fixture(
            "config_test",
            &[
                "src/main.rs",
                "Cargo.toml",
                "Dockerfile",
                ".gitignore",
            ],
        );
        let config = Config::default();
        let scanner = Scanner::new(fixture.path().to_path_buf(), config);
        let result = scanner.scan().unwrap();
        // Config files collected should include Cargo.toml, Dockerfile, .gitignore
        let config_names: Vec<String> = result
            .config_files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(config_names.contains(&"Cargo.toml".to_string()));
        assert!(config_names.contains(&"Dockerfile".to_string()));
        assert!(config_names.contains(&".gitignore".to_string()));
    }

    #[test]
    fn test_lang_detection_rust() {
        let fixture = create_test_fixture(
            "lang_rust",
            &["Cargo.toml", "src/main.rs"],
        );
        let detected = lang::detect_language(fixture.path());
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().name, "Rust");
    }

    #[test]
    fn test_lang_detection_javascript() {
        let fixture = create_test_fixture(
            "lang_js",
            &["package.json", "src/index.js"],
        );
        let detected = lang::detect_language(fixture.path());
        assert!(detected.is_some());
        assert_eq!(detected.unwrap().name, "JavaScript/TypeScript");
    }

    #[test]
    fn test_lang_detection_unknown() {
        let fixture = create_test_fixture(
            "lang_unknown",
            &["README.md", "data.csv"],
        );
        let detected = lang::detect_language(fixture.path());
        assert!(detected.is_none());
    }
}
