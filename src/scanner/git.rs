// Git integration — git log, diff detection, branch info
// Uses the git2 crate for real git repository access.

use std::path::Path;
use std::fmt;

/// Information about the git repository state
#[derive(Debug, Clone)]
pub struct GitInfo {
    pub current_branch: String,
    pub recent_commits: Vec<CommitInfo>,
    pub uncommitted_files: Vec<String>,
    pub last_commit_time: String,
}

/// A single commit entry
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub timestamp: String,
}

impl fmt::Display for CommitInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} <{}> {}", &self.hash[..7.min(self.hash.len())], self.message, self.author, self.timestamp)
    }
}

/// Attempt to get git info for a given repository root.
/// Returns None if the directory is not a git repository or git2 fails.
pub fn get_git_info(root: &Path) -> Option<GitInfo> {
    let repo = git2::Repository::open(root).ok()?;

    let current_branch = get_current_branch(&repo);
    let recent_commits = get_recent_commits(&repo, 10);
    let uncommitted_files = get_uncommitted_files(&repo);
    let last_commit_time = recent_commits
        .first()
        .map(|c| c.timestamp.clone())
        .unwrap_or_default();

    Some(GitInfo {
        current_branch,
        recent_commits,
        uncommitted_files,
        last_commit_time,
    })
}

/// Get the current branch name, or "HEAD (detached)" if not on a branch.
fn get_current_branch(repo: &git2::Repository) -> String {
    match repo.head() {
        Ok(head) => {
            if let Some(name) = head.shorthand() {
                name.to_string()
            } else {
                "HEAD (detached)".to_string()
            }
        }
        Err(_) => "HEAD (detached)".to_string(),
    }
}

/// Get the most recent N commits from the current branch.
fn get_recent_commits(repo: &git2::Repository, count: usize) -> Vec<CommitInfo> {
    let mut revwalk = match repo.revwalk() {
        Ok(walk) => walk,
        Err(_) => return vec![],
    };

    if revwalk.push_head().is_err() {
        return vec![];
    }

    // Sort by time descending (most recent first)
    let _ = revwalk.set_sorting(git2::Sort::TIME);

    let mut commits = Vec::new();
    for oid_result in revwalk {
        if commits.len() >= count {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let hash = oid.to_string();
        let message = commit
            .message()
            .unwrap_or("")
            .lines()
            .next()
            .unwrap_or("")
            .to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let timestamp = commit
            .time()
            .seconds()
            .to_string();

        commits.push(CommitInfo {
            hash,
            message,
            author,
            timestamp,
        });
    }

    commits
}

/// Get a list of files that have uncommitted changes (modified, staged, untracked).
fn get_uncommitted_files(repo: &git2::Repository) -> Vec<String> {
    let mut files = Vec::new();

    // Get diff between index and working directory (unstaged changes)
    if let Ok(diff) = repo.diff_index_to_workdir(None, None) {
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path() {
                let path_str = path.to_string_lossy().to_string();
                if !files.contains(&path_str) {
                    files.push(path_str);
                }
            }
        }
    }

    // Get diff between HEAD and index (staged changes)
    if let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) {
        let mut diff_opts = git2::DiffOptions::new();
        if let Ok(diff) = repo.diff_tree_to_index(Some(&head_tree), None, Some(&mut diff_opts)) {
            for delta in diff.deltas() {
                if let Some(path) = delta.new_file().path() {
                    let path_str = path.to_string_lossy().to_string();
                    if !files.contains(&path_str) {
                        files.push(path_str);
                    }
                }
            }
        }
    }

    // Get untracked files
    let mut status_opts = git2::StatusOptions::new();
    status_opts.include_untracked(true);
    if let Ok(statuses) = repo.statuses(Some(&mut status_opts)) {
        for entry in statuses.iter() {
            if let Some(path) = entry.path() {
                let path_str = path.to_string();
                if !files.contains(&path_str) {
                    files.push(path_str);
                }
            }
        }
    }

    files.sort();
    files
}

/// Get a list of files that have changed since a given reference (e.g., a commit hash or "HEAD~5").
/// Returns file paths relative to the repository root.
pub fn get_changed_files_since(root: &Path, since: &str) -> Vec<String> {
    let repo = match git2::Repository::open(root) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    // Parse the "since" revision
    let since_obj = match repo.revparse_single(since) {
        Ok(obj) => obj,
        Err(_) => return vec![],
    };
    let since_tree = match since_obj.peel_to_tree() {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    // Get the HEAD tree
    let head_tree = match repo.head().and_then(|h| h.peel_to_tree()) {
        Ok(t) => t,
        Err(_) => return vec![],
    };

    let mut files = Vec::new();
    if let Ok(diff) = repo.diff_tree_to_tree(Some(&since_tree), Some(&head_tree), None) {
        for delta in diff.deltas() {
            if let Some(path) = delta.new_file().path() {
                let path_str = path.to_string_lossy().to_string();
                if !files.contains(&path_str) {
                    files.push(path_str);
                }
            }
            if let Some(path) = delta.old_file().path() {
                let path_str = path.to_string_lossy().to_string();
                if !files.contains(&path_str) {
                    files.push(path_str);
                }
            }
        }
    }

    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Create a temporary git repository for testing.
    fn create_git_repo() -> tempfile::TempDir {
        let dir = tempfile::TempDir::with_prefix("git_test_").unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Configure a user for commits
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create an initial commit with a file
        let file_path = dir.path().join("README.md");
        fs::write(&file_path, b"# Test Repo\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("README.md")).unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[]).unwrap();

        dir
    }

    fn add_commit(repo_path: &Path, filename: &str, content: &[u8], message: &str) {
        let repo = git2::Repository::open(repo_path).unwrap();
        let file_path = repo_path.join(filename);

        // Create parent directories if needed
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }

        fs::write(&file_path, content).unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new(filename)).unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();

        let parent_oid = repo.head().unwrap().target().unwrap();
        let parent_commit = repo.find_commit(parent_oid).unwrap();
        let signature = git2::Signature::now("Test User", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &signature, &signature, message, &tree, &[&parent_commit]).unwrap();
    }

    #[test]
    fn test_get_git_info_returns_some_for_git_repo() {
        let dir = create_git_repo();
        let info = get_git_info(dir.path());
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.current_branch, "master");
        assert_eq!(info.recent_commits.len(), 1);
        assert_eq!(info.recent_commits[0].message, "Initial commit");
        assert_eq!(info.recent_commits[0].author, "Test User");
    }

    #[test]
    fn test_get_git_info_returns_none_for_non_git_repo() {
        let dir = tempfile::TempDir::with_prefix("non_git_").unwrap();
        let info = get_git_info(dir.path());
        assert!(info.is_none());
    }

    #[test]
    fn test_get_git_info_recent_commits() {
        let dir = create_git_repo();
        add_commit(dir.path(), "src/main.rs", b"fn main() {}", "Add main.rs");
        add_commit(dir.path(), "src/lib.rs", b"pub fn helper() {}", "Add lib.rs");

        let info = get_git_info(dir.path());
        assert!(info.is_some());
        let info = info.unwrap();
        // 3 commits total: initial + 2 added
        assert_eq!(info.recent_commits.len(), 3);
        // Collect all messages to verify all commits are present
        let messages: Vec<&str> = info.recent_commits.iter().map(|c| c.message.as_str()).collect();
        assert!(messages.contains(&"Add lib.rs"));
        assert!(messages.contains(&"Add main.rs"));
        assert!(messages.contains(&"Initial commit"));
    }

    #[test]
    fn test_get_git_info_uncommitted_files() {
        let dir = create_git_repo();
        // Add an uncommitted file
        fs::write(dir.path().join("uncommitted.txt"), b"changes").unwrap();

        let info = get_git_info(dir.path());
        assert!(info.is_some());
        let info = info.unwrap();
        assert!(info.uncommitted_files.contains(&"uncommitted.txt".to_string()));
    }

    #[test]
    fn test_get_changed_files_since() {
        let dir = create_git_repo();
        add_commit(dir.path(), "src/main.rs", b"fn main() {}", "Add main.rs");

        let changed = get_changed_files_since(dir.path(), "HEAD~1");
        assert!(changed.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_get_changed_files_since_no_changes() {
        let dir = create_git_repo();
        let changed = get_changed_files_since(dir.path(), "HEAD");
        assert!(changed.is_empty());
    }

    #[test]
    fn test_get_changed_files_since_invalid_repo() {
        let dir = tempfile::TempDir::with_prefix("non_git_").unwrap();
        let changed = get_changed_files_since(dir.path(), "HEAD~1");
        assert!(changed.is_empty());
    }

    #[test]
    fn test_commit_info_display() {
        let ci = CommitInfo {
            hash: "abc123def456".to_string(),
            message: "Fix bug".to_string(),
            author: "Test User".to_string(),
            timestamp: "1234567890".to_string(),
        };
        let display = format!("{}", ci);
        assert!(display.contains("abc123d"));
        assert!(display.contains("Fix bug"));
        assert!(display.contains("Test User"));
    }
}
