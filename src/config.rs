use serde::{Deserialize, Serialize};
use std::path::Path;

pub const CONFIG_DIR: &str = ".ctxpkg";
pub const AUTO_DIR: &str = ".ctxpkg/auto";
pub const MANUAL_DIR: &str = ".ctxpkg/manual";
pub const CONFIG_FILE: &str = ".ctxpkg/config.yaml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project: ProjectConfig,
    pub pack: PackConfig,
    pub auto: AutoConfig,
    pub watch: WatchConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackConfig {
    #[serde(default = "default_token_budget")]
    pub token_budget: usize,
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default)]
    pub include_patterns: Vec<String>,
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoConfig {
    #[serde(default = "default_max_files")]
    pub max_files: usize,
    #[serde(default = "default_recent_commits")]
    pub recent_commits: usize,
    #[serde(default = "default_detect_language")]
    pub detect_language: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchConfig {
    #[serde(default = "default_debounce_ms")]
    pub debounce_ms: u64,
}

fn default_token_budget() -> usize {
    8000
}
fn default_format() -> String {
    "markdown".to_string()
}
fn default_max_depth() -> usize {
    4
}
fn default_exclude_patterns() -> Vec<String> {
    vec![
        "*.min.*".into(),
        "*.map".into(),
        "package-lock.json".into(),
        "yarn.lock".into(),
        "pnpm-lock.yaml".into(),
    ]
}
fn default_max_files() -> usize {
    200
}
fn default_recent_commits() -> usize {
    10
}
fn default_detect_language() -> String {
    "auto".to_string()
}
fn default_debounce_ms() -> u64 {
    1000
}

impl Default for Config {
    fn default() -> Self {
        Self {
            project: ProjectConfig {
                name: String::new(),
                description: String::new(),
            },
            pack: PackConfig {
                token_budget: default_token_budget(),
                format: default_format(),
                max_depth: default_max_depth(),
                include_patterns: vec![],
                exclude_patterns: default_exclude_patterns(),
            },
            auto: AutoConfig {
                max_files: default_max_files(),
                recent_commits: default_recent_commits(),
                detect_language: default_detect_language(),
            },
            watch: WatchConfig {
                debounce_ms: default_debounce_ms(),
            },
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Result<Self, crate::error::CtxpkgError> {
        if !path.exists() {
            return Ok(Config::default());
        }
        let contents = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}
