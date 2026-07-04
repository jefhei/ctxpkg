// Git integration — git log, diff detection
// Full implementation with git2 comes in Phase 2.4

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

/// Attempt to get git info for a given repository root.
/// Returns None if the directory is not a git repository.
pub fn get_git_info(root: &std::path::Path) -> Option<GitInfo> {
    // Check if .git directory exists — basic repo detection
    let git_dir = root.join(".git");
    if !git_dir.exists() {
        return None;
    }

    // Placeholder: full git2-based implementation will be added in task 2.4
    // For now, return a minimal structure so scan() compiles and works
    Some(GitInfo {
        current_branch: String::new(),
        recent_commits: vec![],
        uncommitted_files: vec![],
        last_commit_time: String::new(),
    })
}
