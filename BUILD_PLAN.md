# ctxpkg — Build Plan

**Repo:** https://github.com/jefhei/ctxpkg  
**PRD:** `/opt/data/ctxpkg/PRD.md`  
**Language:** Rust  
**Target:** Single binary CLI tool (`ctxpkg`)

---

## How to Use This Plan

Each phase lists individual tasks. Each task should be implemented by **one agent in one session**. Tasks within a phase are ordered by dependency — parallel ones are grouped together.

**Agent working conventions:**
- Commit after each completed task with a descriptive message
- Open a PR per phase or per logical group of tasks
- Run `cargo build` and `cargo test` before marking any task done
- If stuck on a task for >3 attempts, escalate it — don't block the phase

---

## Phase 0 — Repo & Scaffold ✅ (done)

- [x] Create GitHub repo `jefhei/ctxpkg`
- [x] Write PRD at `/opt/data/ctxpkg/PRD.md`

---

## Phase 1 — Core Scaffolding

**Goal:** Runnable skeleton with CLI, config, and error handling.

### 1.1 Initialize project

- Run `cargo init --name ctxpkg` in the repo root
- Create `src/` directory structure:

```
src/
├── main.rs
├── cli.rs            # CLI argument parsing (clap)
├── config.rs         # Config struct + loading
├── error.rs          # Error types
├── scanner/
│   └── mod.rs        # (placeholder)
├── detect/
│   └── mod.rs        # (placeholder)
├── packer.rs         # (placeholder)
└── clipboard.rs      # (placeholder)
```

### 1.2 Set up Cargo.toml

```toml
[package]
name = "ctxpkg"
version = "0.1.0"
edition = "2021"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
anyhow = "1"
thiserror = "1"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
```

### 1.3 Implement `main.rs`

- Parse CLI args, dispatch to command handlers
- Print help by default
- Structure:

```rust
mod cli;
mod config;
mod error;
mod scanner;
mod detect;
mod packer;
mod clipboard;

fn main() -> anyhow::Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Command::Init { path } => cmd_init(path),
        cli::Command::Pack { output, token_budget, format } => cmd_pack(output, token_budget, format),
        cli::Command::Inject { token_budget } => cmd_inject(token_budget),
        cli::Command::Status { verbose } => cmd_status(verbose),
        cli::Command::Graft { pattern } => cmd_graft(pattern),
    }
}
```

### 1.4 Implement `cli.rs`

- Use `clap::Parser` derive macro
- Commands: `Init`, `Pack`, `Inject`, `Status`, `Graft`
- Each command struct with its flags/args per PRD section 4.1
- Global `--quiet` and `--json` flags

### 1.5 Implement `error.rs`

- Custom error enum with `thiserror`:
  - `ConfigError(String)` — malformed config, missing required fields
  - `ScanError(String)` — filesystem errors during scanning
  - `DetectError(String)` — unsupported or broken language detection
  - `PackError(String)` — assembly failures
  - `ClipboardError(String)` — clipboard access failures
- `Display` impls with user-friendly messages
- `From` impls for `std::io::Error`, `serde_yaml::Error`

### 1.6 Implement `config.rs`

- `Config` struct with all fields from PRD section 4.4
- `Config::load(path: &Path) -> Result<Self>` — reads YAML, merges with defaults
- `Config::default() -> Self` — sensible defaults
- Include module-level constants for default paths (`.ctxpkg/config.yaml`, `.ctxpkg/auto/`, `.ctxpkg/manual/`)

### 1.7 Verify: `cargo build` + `cargo test`

- All modules compile
- `ctxpkg --help` shows all commands
- `cargo test` passes (even if only placeholder tests)

---

## Phase 2 — Scanner

**Goal:** Walk a project directory, discover structure, and extract git info.

### 2.1 Implement `scanner/mod.rs`

- `Scanner` struct holding project root path + config
- `Scanner::new(path, config) -> Self`
- `Scanner::scan() -> Result<ScanResult>`

### 2.2 Define `ScanResult`

```rust
pub struct ScanResult {
    pub root: PathBuf,
    pub tree: DirectoryTree,
    pub file_count: usize,
    pub detected_language: Option<String>,
    pub build_system: Option<String>,
    pub config_files: Vec<PathBuf>,
    pub source_files: Vec<PathBuf>,
    pub git_info: Option<GitInfo>,
}
```

### 2.3 Define `DirectoryTree`

- Recursive structure with depth cap (from config)
- `TreeNode` enum: `File { name, path, size }` | `Dir { name, children, file_count }`
- `DirectoryTree::build(root, max_depth, exclude_patterns) -> Result<Self>`
- Use the `ignore` crate for `.gitignore`-aware walking
- Skip: `node_modules`, `target`, `.git`, `vendor`, `__pycache__`, `.ctxpkg`

### 2.4 Git integration (`scanner/git.rs`)

- Wrap git2 crate calls
- `GitInfo` struct: `current_branch`, `recent_commits: Vec<CommitInfo>`, `uncommitted_files: Vec<String>`, `last_commit_time`
- `CommitInfo`: `hash`, `message` (first line), `author`, `timestamp`
- `fn get_git_info(root: &Path) -> Option<GitInfo>` — return None if not a git repo
- `fn get_changed_files_since(root: &Path, since: &str) -> Vec<String>` — for diff-aware prioritization

### 2.5 Language detection framework (`scanner/lang.rs`)

- `DetectedLanguage` struct: `name`, `version` (optional), `build_files`, `detector_used`
- `fn detect_language(scan_result: &ScanResult) -> Option<DetectedLanguage>`
- Heuristic-based: check for `Cargo.toml` → Rust, `package.json` → JS/TS, `pyproject.toml`/`setup.py` → Python, `go.mod` → Go, `Gemfile` → Ruby, else generic
- Keep it simple — returns the primary language, not a mix

### 2.6 Add deps to Cargo.toml

```toml
[dependencies]
ignore = "0.4"
git2 = "0.19"
walkdir = "2"
```

### 2.7 Verify

- `cargo test` passes
- Unit tests for tree building with mock directory fixtures
- Test depth limiting, exclude patterns, empty directories

---

## Phase 3 — Language Detectors

**Goal:** Extract API surface, dependency summaries, and key structure per language.

### 3.1 Language detector trait

`detect/mod.rs`:

```rust
pub trait LanguageDetector {
    fn name(&self) -> &'static str;
    fn detect(&self, files: &[PathBuf]) -> bool;
    fn extract_api_surface(&self, files: &[PathBuf]) -> Result<Vec<ApiSymbol>>;
    fn extract_deps(&self, files: &[PathBuf]) -> Result<Vec<Dependency>>;
    fn extract_configs(&self, files: &[PathBuf]) -> Result<Vec<ConfigFile>>;
}

pub struct ApiSymbol {
    pub name: String,
    pub kind: SymbolKind,  // Function, Struct, Class, Type, Route, Endpoint
    pub file: PathBuf,
    pub line: usize,
    pub visibility: Visibility,  // Public, Private, Exported
}

pub enum SymbolKind { Function, Struct, Class, Trait, Enum, Type, Constant, Route }
pub enum Visibility { Public, Exported, Private }
pub struct Dependency { pub name: String, pub version: Option<String>, pub purpose: Option<String> }
pub struct ConfigFile { pub path: PathBuf, pub content_summary: String }
```

### 3.2 Python detector (`detect/python.rs`)

- Detection: presence of `pyproject.toml`, `setup.py`, `requirements.txt`, `Pipfile`
- API surface: walk `.py` files, use regex or simple AST to find `def`, `class`, async def at module level
- Dependencies: parse `pyproject.toml` (toml), `requirements.txt`
- Build system: pip, poetry, uv, pdm

### 3.3 JavaScript/TypeScript detector (`detect/javascript.rs`)

- Detection: presence of `package.json`
- API surface: parse exported symbols from `.ts`/`.tsx`/`.js`/`.jsx` files
  - `export function`, `export class`, `export const`, `export default`
  - Express/Fastify route patterns (`app.get(`, `router.post(`)
- Dependencies: parse `package.json` deps + devDeps
- Build system: npm, yarn, pnpm detection via lockfile presence

### 3.4 Rust detector (`detect/rust.rs`)

- Detection: `Cargo.toml`
- API surface: walk `.rs` files, extract `pub fn`, `pub struct`, `pub enum`, `pub trait`, `pub mod`, `pub type`
- Dependencies: parse `Cargo.toml` for `[dependencies]`
- Build system: cargo

### 3.5 Go detector (`detect/golang.rs`)

- Detection: `go.mod`
- API surface: exported functions/types (capital-letter naming convention), parse `func`, `type`, `struct`
- Dependencies: parse `go.mod` require block
- Build system: go

### 3.6 Generic detector (`detect/generic.rs`)

- Fallback when no specific language detected
- No API surface extraction
- File count by extension, largest files, directory structure
- Detect potential build files: `Makefile`, `Dockerfile`, `docker-compose.yml`, `Justfile`, `Taskfile.yml`

### 3.7 Registry in `detect/mod.rs`

- `fn all_detectors() -> Vec<Box<dyn LanguageDetector>>`
- Ordered by specificity (Rust > Go > Python > JS > Ruby > Generic)
- `fn detect_and_extract(files, root) -> Result<LanguageContext>`
- `LanguageContext` struct aggregating all detected languages

### 3.8 Add deps

```toml
[dependencies]
toml = "0.8"              # Python pyproject.toml parsing
regex = "1"               # regex-based API extraction
once_cell = "1"           # lazy static compiled regexes
```

### 3.9 Verify

- Unit tests per detector with known file content
- Test Rust detector against a small Cargo.toml + mod.rs
- Test Python detector against pyproject.toml + module
- Test JS detector against package.json + index.ts
- Edge cases: empty projects, projects with no source files, mixed-language projects

---

## Phase 4 — Packer

**Goal:** Assemble context sections into a token-budgeted, formatted document.

### 4.1 Implement `packer.rs`

- `Packer` struct: `config: Config, scan: ScanResult, lang_ctx: LanguageContext`
- `Packer::new(config, scan, lang_ctx) -> Self`
- `Packer::pack() -> Result<PackedContext>`

### 4.2 Define context sections

```rust
pub struct PackedContext {
    pub sections: Vec<ContextSection>,
    pub total_tokens: usize,
    pub budget: usize,
    pub truncated: Vec<String>,  // names of truncated sections
}

pub struct ContextSection {
    pub title: &'static str,
    pub content: String,
    pub priority: u8,        // 0=essential, 1=high, 2=medium, 3=low
    pub estimated_tokens: usize,
}
```

### 4.3 Section builders

Implement each as a method on `Packer`:

- `build_project_identity()` — name, description, language, build system, version
- `build_directory_tree()` — condensed tree from `ScanResult.tree`, depth-limited
- `build_dependency_snapshot()` — from `LanguageContext`, exclude transitive deps
- `build_api_surface()` — formatted with kind prefix (`fn`, `class`, `route`)
- `build_recent_git_activity()` — branch, recent commits, uncommitted changes
- `build_key_configs()` — build, CI, linter, test configs
- `build_architecture()` — read from `.ctxpkg/manual/architecture.md` if present
- `build_conventions()` — read from `.ctxpkg/manual/conventions.md` if present
- `build_known_issues()` — read from `.ctxpkg/manual/gotchas.md` if present

### 4.4 Token estimation

```rust
fn estimate_tokens(text: &str) -> usize {
    // Simple char-based estimator: 4 chars ≈ 1 token
    // Future: tiktoken-rs for accurate counting
    text.len() / 4 + 1
}
```

### 4.5 Budget algorithm

```rust
fn apply_budget(&self, sections: Vec<ContextSection>, budget: usize) -> PackedContext {
    // 1. Essential sections always included (priority 0)
    // 2. Calculate remaining budget
    // 3. Sort remaining by priority, then by token efficiency (info per token)
    // 4. Include until budget exhausted
    // 5. Track truncated sections
    // 6. Append summary footer
}
```

### 4.6 Output formatting

Two formats:

**Markdown (default):**
```markdown
# ctxpkg — Context Package
Project: my-api (TypeScript)
...

## Structure
...
```

**Text (plain):**
```
# ctxpkg — Context Package
Project: my-api (TypeScript)
...
```

### 4.7 Footer

Every packed context ends with:
```
---
[Context: X tokens of Y budget | Sections: N included, M truncated]
```

### 4.8 Verify

- Unit test: assemble known sections, verify token count and budget enforcement
- Test with budget=0 (unlimited) — all sections included
- Test with tiny budget=500 — only essential sections
- Verify markdown and text output formats

---

## Phase 5 — Commands

**Goal:** Wire all commands to produce real output.

### 5.1 `cmd_init` in `main.rs`

```rust
fn cmd_init(path: Option<PathBuf>) -> Result<()> {
    // 1. Resolve path (default: current dir)
    // 2. Run scanner to detect project type
    // 3. Create .ctxpkg/ directory structure
    // 4. Write config.yaml with detected settings
    // 5. Create manual/ directory with template files
    // 6. Auto-detect and write initial manual content
    // 7. Print summary:
    //    "✓ ctxpkg initialized in /path/to/project
    //     Project type: TypeScript (Express + Prisma)
    //     Files tracked: 142
    //     Try: ctxpkg inject"
    // 8. Return Ok
}
```

Template files for `manual/`:
- `architecture.md`: "<!-- Describe your project's architecture, key design decisions, and data flow -->"
- `conventions.md`: "<!-- Coding conventions, naming patterns, testing approach -->"
- `gotchas.md`: "<!-- Known pitfalls, gotchas, and things to watch out for -->"

### 5.2 `cmd_pack`

```rust
fn cmd_pack(output: Option<PathBuf>, token_budget: Option<usize>, format: OutputFormat) -> Result<()> {
    // 1. Load config from .ctxpkg/config.yaml
    // 2. Run scanner
    // 3. Run language detection
    // 4. Run packer with budget override if provided
    // 5. Write to output (or stdout)
    // 6. Print token count to stderr
}
```

### 5.3 `cmd_inject`

```rust
fn cmd_inject(token_budget: Option<usize>) -> Result<()> {
    // 1. Run pack internally (same as cmd_pack)
    // 2. Copy to clipboard
    // 3. Print: "✓ Context injected to clipboard (4,230 tokens)"
    // 4. If clipboard unavailable, print: "⚠ Clipboard unavailable. Use 'ctxpkg pack' instead."
}
```

### 5.4 `cmd_status`

```rust
fn cmd_status(verbose: bool) -> Result<()> {
    // 1. Load config, run scanner
    // 2. Print formatted status:
    //    "✓ Project: my-api (TypeScript, Express + Prisma)
    //     ✓ Context size: 4,230 tokens / 8,000 budget
    //     ✓ Last pack: 2 minutes ago
    //     ✓ Manual sections: all present (architecture, conventions)
    //     → 3 files changed since last pack"
    // 3. If verbose, show per-section breakdown
    // 4. Exit non-zero if context is stale (>1 day since last pack)
    // 5. Support --json flag for programmatic consumption
}
```

### 5.5 `cmd_graft`

```rust
fn cmd_graft(pattern: String) -> Result<()> {
    // 1. Resolve glob pattern against project root
    // 2. Add matching files to include_patterns in config
    // 3. Print: "✓ 3 files added to context: src/auth/*.rs, src/middleware/*.rs"
    // 4. If no files match, print error
}
```

### 5.6 Add missing Cargo.toml deps

```toml
[dependencies]
arboard = "3"       # Cross-platform clipboard
chrono = "0.4"      # Timestamps
colored = "2"       # Terminal colors
```

### 5.7 Implement clipboard (`clipboard.rs`)

```rust
pub fn copy_to_clipboard(text: &str) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new()?;
    clipboard.set_text(text)?;
    Ok(())
}
```

Return helpful error on headless systems: "Clipboard not available in headless environment. Use `ctxpkg pack --output context.md` instead."

### 5.8 Verify

- Integration test: `ctxpkg init` on a test directory, verify files created
- `ctxpkg pack` produces output matching expected format
- `ctxpkg status` exit codes correct
- `ctxpkg inject` on non-headless (CI skip)
- `ctxpkg graft src/*.rs` updates config correctly

---

## Phase 6 — Integration

**Goal:** Watch mode, pre-commit hook installer, MCP server mode.

### 6.1 Watch mode (`cmd_watch`)

- New command: `ctxpkg watch [--debounce N]`
- Uses `notify` crate for filesystem events
- Debounce configurable (default 1000ms)
- On file change: re-pack and optionally re-inject
- Print: "⏳ Rebuilding context... (3 files changed)"
- Keep running until Ctrl+C

```toml
[dependencies]
notify = "7"    # File watcher
```

### 6.2 Pre-commit hook installer

- `ctxpkg init --hooks` — installs pre-commit hook
- Writes `.git/hooks/pre-commit` that runs `ctxpkg pack --output .ctxpkg/auto/context.md`
- Detects if hooks dir exists, creates if needed
- Doesn't overwrite existing hooks without `--force`

### 6.3 MCP server mode (stretch)

- `ctxpkg mcp` — starts an MCP server that provides project context as a tool
- Other AI tools can call `get_project_context` to receive the packed context
- Optional: skip for MVP, document as "future work"

### 6.4 Verify

- `ctxpkg watch`: create file, verify context regenerates
- `--hooks` correctly installs hook file
- Test hook runs without error (manual test)

---

## Phase 7 — Quality

**Goal:** Comprehensive test suite, CI pipeline, benchmarks.

### 7.1 Unit tests per module

- **config.rs:** load valid/invalid/missing config, merge defaults, field override
- **scanner:** tree depth, exclude patterns, empty dirs, symlinks (skip)
- **detect:** each language detector against known file patterns
- **packer:** token estimation, budget enforcement, section ordering, format output
- **clipboard:** graceful fallback on headless

### 7.2 Integration tests with fixtures

Create `tests/fixtures/` with small test projects:

```
tests/fixtures/
├── python-project/
│   ├── pyproject.toml
│   └── src/
│       └── module.py
├── js-project/
│   ├── package.json
│   └── src/
│       └── index.ts
├── rust-project/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
├── go-project/
│   ├── go.mod
│   └── main.go
└── empty-project/
    └── README.md
```

Integration tests:
- Run `ctxpkg init` on each fixture, verify file structure
- Run `ctxpkg pack` on each, verify section presence
- Run `ctxpkg status` on each, verify exit code
- Test token budget override (`--token-budget 500`)

### 7.3 Test utilities

- `TestProject::new(name) -> TestProject` — creates temp dir with fixture files
- Auto-cleanup on Drop

### 7.4 CI pipeline (`.github/workflows/ci.yml`)

```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rust-lang/setup-rust-toolchain@v1
      - run: cargo build --verbose
      - run: cargo test --verbose
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

### 7.5 Benchmarks

- Use `criterion` crate for basic benchmarks
- Benchmark: `pack` on a large fixture (1000+ files)
- Target: `pack < 500ms` for typical projects

```toml
[dev-dependencies]
criterion = { version = "0.5", features = ["html_reports"] }

[[bench]]
name = "pack"
harness = false
```

### 7.6 Verify

- `cargo test` passes all tests
- `cargo clippy` has zero warnings (or documented exceptions)
- `cargo fmt --check` passes
- Benchmarks run without errors

---

## Phase 8 — Documentation & Release

**Goal:** Ship it.

### 8.1 README.md

Sections:
- **What is ctxpkg?** — 2-sentence intro with asciinema demo link (placeholder)
- **Quickstart** — install + first 3 commands
- **Install** — shell/cargo/brew options (brew as future)
- **Commands** — reference table
- **How it works** — brief architecture overview
- **Configuration** — config.yaml reference
- **Manual context** — writing architecture.md, conventions.md, gotchas.md
- **Integration** — pre-commit hooks, CI, daily workflow
- **Contributing** — dev setup, how to add a language detector
- **License** — MIT

### 8.2 Install script

`scripts/install.sh` — curl-friendly one-liner:

```bash
curl -fsSL https://raw.githubusercontent.com/jefhei/ctxpkg/main/scripts/install.sh | bash
```

- Detect OS + arch
- Download from GitHub releases
- Install to `/usr/local/bin` or `~/.local/bin`
- Verify checksum if available

### 8.3 GitHub release workflow (`.github/workflows/release.yml`)

```yaml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        target: [x86_64-unknown-linux-gnu, aarch64-apple-darwin, x86_64-pc-windows-msvc]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - run: cargo build --release --target ${{ matrix.target }}
      - run: tar czf ctxpkg-${{ matrix.target }}.tar.gz -C target/${{ matrix.target }}/release ctxpkg
      - uses: softprops/action-gh-release@v2
        with:
          files: ctxpkg-*.tar.gz
```

### 8.4 CHANGELOG.md

- Keep a simple changelog
- v0.1.0: "MVP — init, pack, inject, status, graft. Python/JS/Rust/Go support."

### 8.5 Verify

- `cargo build --release` succeeds
- Binary produces correct output
- README renders correctly on GitHub
- Install script works on a test machine

---

## Appendix: Dependency Summary (Final Cargo.toml)

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
anyhow = "1"
thiserror = "1"
ignore = "0.4"
git2 = "0.19"
walkdir = "2"
toml = "0.8"
regex = "1"
once_cell = "1"
arboard = "3"
chrono = "0.4"
colored = "2"
notify = "7"

[dev-dependencies]
tempfile = "3"
assert_cmd = "2"
criterion = { version = "0.5", features = ["html_reports"] }
```

---

## Appendix: Git Commit Convention

```
feat: add <feature>
fix: fix <bug>
docs: update <documentation>
test: add <test>
refactor: <description>
chore: <maintenance task>
ci: <CI config change>
```

Example sequence:
```
feat: implement project scanner with depth limiting
feat: add Python language detector
feat: wire init command to scanner + config creation
test: add integration tests for init on all fixture types
docs: write README quickstart section
ci: set up GitHub Actions CI
```
