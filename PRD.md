# ctxpkg — Product Requirements Document

**Status:** Draft v1  
**Date:** 2026-07-04  
**Author:** Hermes Agent (for Jeff Heidelberger)

---

## 1. Executive Summary

**ctxpkg** (Context Packages) is a CLI tool that creates, manages, and injects structured project context snapshots optimized for AI coding assistants. It solves the cold-start problem: every session with Claude Code, Gemini CLI, Codex, Cursor, or similar tools starts with zero knowledge of your codebase. Users waste time re-pasting project overviews, architecture notes, and key files. ctxpkg eliminates that friction with a single command.

**Tagline:** *One command. Your project, known.*

---

## 2. Problem Statement

### 2.1 The Gap

AI coding assistants are powerful but stateless across sessions. Each new conversation is a blank slate. The existing workarounds all have tradeoffs:

| Approach | Drawback |
|----------|----------|
| Manually copy-pasting project info | Inconsistent, incomplete, time-consuming |
| Keeping a PROJECT_CONTEXT.md in the repo | Static, goes stale, no prioritization |
| RAG / vector databases | Infrastructure overhead, often sends code to third-party APIs |
| MCP servers | Client-dependent, requires setup per tool, not universal |
| IDE plugins | Locked to one editor, don't work in SSH/headless |

### 2.2 Target Audience

- Solo developers and small teams using AI coding assistants daily
- Developers working in headless/SSH environments (no IDE plugins available)
- Anyone who switches between AI tools (Claude Code, Gemini CLI, Codex, Cursor, etc.)
- Open-source maintainers who want contributors to interact with their codebase through AI tools effectively

### 2.3 Use Cases

1. **Daily development:** `ctxpkg inject` before starting a coding session so the AI already knows the project
2. **Context switching:** Jump between projects — `ctxpkg pack` in each captures the relevant state
3. **Onboarding:** Share a `.ctxpkg/manual/` directory in the repo so new contributors give the AI the right context
4. **CI integration:** Pre-commit hook regenerates context so it always reflects current state
5. **PR handoff:** When switching a PR between AI tools, inject the context on both sides

---

## 3. Product Overview

### 3.1 Description

ctxpkg is a single-binary CLI tool (written in Rust) that:

1. **Scans** a project directory to automatically discover structure, dependencies, API surface, and git history
2. **Assembles** a token-efficient context document prioritizing what changed recently and what's most relevant to AI understanding
3. **Delivers** that context to clipboard, stdout, or a file — ready to paste into any AI tool

It also supports **user-authored context** (architecture docs, conventions, known gotchas) that persists and is merged into each pack.

### 3.2 Positioning

- **Not** an MCP server — works with every AI tool, not just MCP-compatible ones
- **Not** a RAG system — no vector DBs, no embeddings, no API calls to third parties
- **Not** an IDE plugin — works in any terminal, including SSH/headless
- **Is** a universal, zero-dependency context packer that any AI tool can consume

### 3.3 Core Principles

1. **Local-first:** Everything runs on your machine. No data ever leaves.
2. **Zero config to start:** `ctxpkg init` with no arguments produces a useful context for most projects.
3. **Progressive enhancement:** Add `.ctxpkg/config.yaml` and `.ctxpkg/manual/` files as you need more control.
4. **Git-aware:** Context priority is driven by what changed recently.
5. **Token-conscious:** Default output stays under 8K tokens. User can set their own budget.
6. **Universal:** Works with any AI tool, any language, any framework.

---

## 4. Features & Requirements

### 4.1 CLI Commands (MVP v1)

#### `ctxpkg init [path]`

- Scans the project at `path` (default: `.`)
- Creates `.ctxpkg/auto/` directory with generated context files
- Creates `.ctxpkg/manual/` directory with templates for user-authored docs
- Creates `.ctxpkg/config.yaml` with sensible defaults
- Adds `.ctxpkg/` to `.gitignore` (only `manual/` is committed)
- Outputs a summary of what was discovered

#### `ctxpkg pack [--output FILE] [--token-budget N] [--format text|markdown]`

- Reads current state from `.ctxpkg/auto/` and `.ctxpkg/manual/`
- Checks git diff to weight recent changes higher
- Assembles a single context document
- Stays within the token budget (default: 8K tokens, use `--token-budget 0` for unlimited)
- Writes to stdout if no `--output` given

#### `ctxpkg inject [--token-budget N]`

- Runs `pack` internally
- Copies result to system clipboard (cross-platform: macOS pbcopy, Linux xclip/wayland, Windows clip)
- Prints token count and brief summary of what's included

#### `ctxpkg status [--verbose]`

- Shows: project type, file count, context size (tokens), number of sections
- Highlights: files that changed since last pack, missing manual sections
- Exits non-zero if context is stale (useful for CI gating)

#### `ctxpkg graft <file-or-glob>`

- Manually adds a file or pattern to the context
- Useful for files the auto-scanner might miss or that deserve higher priority

### 4.2 Supported Languages/Frameworks (v1 Auto-Detection)

| Language | What it discovers |
|----------|-------------------|
| Python | pyproject.toml, requirements.txt, module structure, public API via ast |
| JavaScript/TypeScript | package.json, exports, tsconfig paths, exported functions/types |
| Rust | Cargo.toml, module tree, public API via syn |
| Go | go.mod, package structure, exported symbols |
| Ruby | Gemfile, module structure |
| Generic | File tree, git log, key config files (Dockerfile, CI, Makefile) |

Each language detector is a separate module — easy to add more.

### 4.3 Context Sections (what goes into the pack)

The assembled context document contains these sections, in order:

1. **Project Identity** — name, description, language, build system, key version
2. **Directory Tree** — condensed tree view (depth-limited, exclude node_modules/.git/target)
3. **Dependency Snapshot** — key dependencies and their purposes (not the full lockfile)
4. **API Surface** — exported functions, types, classes, routes, endpoints
5. **Architecture** — from `.ctxpkg/manual/architecture.md` if present, else auto-inferred from structure
6. **Conventions** — from `.ctxpkg/manual/conventions.md` if present
7. **Recent Git Activity** — last 10 commits, current branch, uncommitted changes
8. **Key Configs** — build config, CI config, linter rules, test setup
9. **Known Issues** — from `.ctxpkg/manual/gotchas.md` if present

### 4.4 Configuration (config.yaml)

```yaml
project:
  name: ""  # auto-detected, can override
  description: ""  # optional, goes into context

pack:
  token_budget: 8000  # 0 = unlimited
  format: markdown     # text or markdown
  max_depth: 4         # max directory tree depth
  include_patterns: [] # extra globs to include
  exclude_patterns:    # always excluded
    - "*.min.*"
    - "*.map"
    - "package-lock.json"
    - "yarn.lock"
    - "pnpm-lock.yaml"

auto:
  max_files: 200       # cap for auto-scan
  recent_commits: 10   # git log depth
  detect_language: auto # or force "python", "javascript", etc.

watch:
  debounce_ms: 1000     # debounce for watch mode
```

---

## 5. Technical Architecture

### 5.1 Language Choice: Rust

| Criterion | Rust | Python | Go |
|-----------|------|--------|-----|
| Single binary | ✅ Native | ❌ Requires runtime | ✅ Native |
| Startup time | ~1ms | ~50ms | ~5ms |
| Cross-platform | ✅ | ✅ (with runtime) | ✅ |
| Ecosystem for AST parsing | ✅ (syn, tree-sitter) | ✅ (ast module) | ❌ Limited |
| Memory safety | ✅ | N/A (interpreted) | ✅ |

Rust wins on startup speed (critical for a tool you run in your git hook) and zero-dependency distribution.

### 5.2 Key Dependencies (Rust)

- `clap` — CLI argument parsing
- `tokei` — Language detection and file counting
- `serde` + `serde_yaml` — Config parsing
- `ignore` (ripgrep crate) — File walking with .gitignore support
- `git2` — Git integration
- `syntect` or `tree-sitter` — Language parsing for API surface extraction
- `arboard` — Cross-platform clipboard
- `tiktoken-rs` — Token counting (matches OpenAI/Claude tokenizers)

### 5.3 File Layout

```
ctxpkg/
├── Cargo.toml
├── src/
│   ├── main.rs              # CLI entry point, command dispatch
│   ├── config.rs            # Config loading & defaults
│   ├── scanner/             # Project scanning
│   │   ├── mod.rs
│   │   ├── tree.rs          # Directory tree builder
│   │   ├── git.rs           # Git log, diff detection
│   │   └── deps.rs          # Dependency parsing
│   ├── detect/              # Language detectors
│   │   ├── mod.rs
│   │   ├── python.rs
│   │   ├── javascript.rs
│   │   ├── rust.rs
│   │   ├── golang.rs
│   │   └── generic.rs
│   ├── packer.rs            # Context assembly & token budgeting
│   ├── clipboard.rs         # Clipboard cross-platform
│   └── watch.rs             # File watcher
├── tests/
│   ├── fixtures/            # Test project templates
│   └── integration.rs
└── .ctxpkg/                 # ctxpkg's own context (dogfooding)
    ├── config.yaml
    └── manual/
        └── architecture.md
```

### 5.4 Token Budgeting Algorithm

```
Input: All context sections, each with estimated token count
Input: Budget B (default 8000)

Algorithm:
1. Essential sections (Project Identity, Directory Tree, Dependency Snapshot) always included
   - Total essential ~1500 tokens
2. Remaining budget = B - essential
3. Rank remaining sections by priority score:
   - Recently modified files (from git diff) → higher priority
   - API surface → higher priority than configs
   - User-authored manual sections → always priority 1
4. Include sections in priority order until budget is exhausted
5. If budget is 0 (unlimited), include everything
6. Append a note showing token count and any truncated sections
```

---

## 6. User Experience

### 6.1 Getting Started

```bash
# Install (one line)
curl -fsSL https://ctxpkg.dev/install.sh | bash

# Or via cargo
cargo install ctxpkg

# Or via brew
brew install ctxpkg/ctxpkg

# Use
cd my-project
ctxpkg init            # Auto-discovers project structure
ctxpkg inject          # Generates context, copies to clipboard
# Paste into Claude Code / Gemini CLI / Codex / Cursor
```

### 6.2 Workflow Integration

**Daily session start:**
```bash
cd my-project
ctxpkg inject  # 500ms later, context is in your clipboard
# Open Claude Code, paste context, start coding
```

**Pre-commit hook (in .git/hooks/pre-commit):**
```bash
#!/bin/sh
ctxpkg pack --output .ctxpkg/auto/context.md
git add .ctxpkg/auto/context.md
```

**CI check (in CI pipeline):**
```bash
ctxpkg status --verbose
# Exits non-zero if context is more than 1 day stale
```

### 6.3 Visual Style

- Clean, modern CLI output with colored sections
- Progress spinner for scanning operations (>500ms)
- Clear error messages with suggested fixes
- `--quiet` flag for script usage
- `--json` flag for programmatic consumption

---

## 7. Release Criteria

### 7.1 MVP v1 (target: 2 weeks)

- [ ] `ctxpkg init` works for Python, JavaScript/TypeScript, Rust, Go, and generic projects
- [ ] `ctxpkg pack` produces a coherent context under 8K tokens
- [ ] `ctxpkg inject` copies to clipboard on macOS, Linux (xclip + wl-copy), and Windows
- [ ] `ctxpkg status` shows useful information
- [ ] `ctxpkg graft` works for manual additions
- [ ] Config via `.ctxpkg/config.yaml`
- [ ] Manual context via `.ctxpkg/manual/*.md`
- [ ] Git-aware prioritization (weight recent changes higher)
- [ ] Token budget enforcement
- [ ] Tests for core assembly, token counting, and at least one language detector
- [ ] README with install instructions and quickstart
- [ ] GitHub release with pre-built binaries (Linux x86_64, macOS aarch64)

### 7.2 v1.1 (nice-to-have, post-MVP)

- [ ] `ctxpkg watch` mode with file watcher
- [ ] Tree-sitter based API surface extraction (deeper than regex/ast)
- [ ] Pre-commit hook auto-install via `ctxpkg init --hooks`
- [ ] GitHub Actions integration (comment PRs with stale context warnings)
- [ ] MCP server mode (`ctxpkg mcp` — acts as an MCP tool providing context)
- [ ] Homebrew tap + cargo publish

### 7.3 Quality Gates

- **Performance:** `init` completes in <2s for projects with <10K files. `pack` completes in <200ms after init.
- **Correctness:** Parsed API surface must match what the language's own tools report for at least 90% of exports.
- **Token efficiency:** Packed context must contain no redundant information (duplicate file listings, overlapping descriptions).
- **Portability:** Binaries must run on macOS 12+, Ubuntu 20.04+, Windows 10+ without any system dependencies.

---

## 8. Success Metrics

| Metric | Target |
|--------|--------|
| Time saved per session | ≥2 minutes (vs. manual context setup) |
| Init-to-inject latency | <3 seconds for most projects |
| GitHub stars (3 months) | >500 |
| Active users (weekly) | >50 |
| Context document size | Always <15K tokens at default budget |
| Language coverage | 5 languages at MVP, 10+ by v1.1 |

---

## 9. Competitive Landscape

| Tool | Type | Why not ctxpkg |
|------|------|----------------|
| Manual CONTEXT.md | Static file | Goes stale, no auto-regeneration, no token awareness |
| screenpipe (19k ⭐) | Screen recording → context | Desktop-only, heavy, solves a different problem |
| MCP servers (awesome-mcp-servers 90k ⭐) | Protocol | Requires MCP-compatible client, per-tool setup |
| Claude Code projects | Anthropic-only | Locked to Claude Code, no custom context control |
| Repomix/repo2txt | Repo → single file | No token budgeting, no git awareness, no manual sections |
| Copilot Chat / Cody | IDE plugin | Locked to editors, no SSH/headless use |
| code2prompt (2k ⭐) | CLI context builder | Less structured output, no git-aware prioritization, no config |

**Ctxpkg differentiators:** universal (works with any AI tool), git-aware prioritization, token budgeting, progressive enhancement (zero-config → fully customized), local-first.

---

## 10. Roadmap

```
Week 1:
  - Project scaffold in Rust
  - `init` command: directory scanner, language detection, config creation
  - `pack` command: basic section assembly, token counting
  - `inject` command: clipboard integration
  - Python + JS/TS language detectors

Week 2:
  - Rust + Go language detectors
  - Git-aware prioritization
  - `status` + `graft` commands
  - Manual context sections
  - Token budget enforcement
  - Tests + CI
  - README + GitHub release with binaries

Post-MVP:
  - `watch` mode
  - Tree-sitter deep parsing
  - Homebrew tap
  - Cargo publish
  - MCP server mode
```

---

## 11. Open Questions

1. **Token counting:** Use `tiktoken-rs` (OpenAI/Claude tokenizer) or a simpler character-based estimate? tiktoken is accurate but heavier. A character estimate (4 chars ≈ 1 token) is faster but imprecise.
   → **Decision:** Use tiktoken-rs for accuracy, with a fast-path char estimate when tiktoken isn't available.

2. **Cache invalidation:** When should context auto-regenerate? On git commit? On file save? On demand only?
   → **Decision:** On demand by default. Git hook integration is opt-in.

3. **Large monorepos:** How to handle projects with 50K+ files? Directory tree alone could exceed token budget.
   → **Decision:** `exclude_patterns` defaults + user-configured `include_patterns`. Tree depth capped at 4 by default.

4. **Binary distribution:** GitHub releases + shell installer vs. package manager distribution?
   → **Decision:** GitHub releases with `install.sh` for v1. Homebrew + cargo after v1.1.

---

## 12. Appendix: Example Output

```
$ ctxpkg pack
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  ctxpkg — Context Package
  Project: my-api (TypeScript)
  2026-07-04T14:30:00Z
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

▸ PROJECT
  my-api v2.1.0 — Express API server with PostgreSQL

▸ STRUCTURE
  src/
  ├── routes/        (7 files)
  ├── middleware/     (3 files)
  ├── models/        (5 files)
  └── services/      (4 files)

▸ DEPENDENCIES
  express, prisma, zod, vitest, swagger-jsdoc

▸ API SURFACE
  GET    /api/v1/users         → listUsers(params)
  POST   /api/v1/users         → createUser(body)
  GET    /api/v1/users/:id     → getUser(id)
  PATCH  /api/v1/users/:id     → updateUser(id, body)
  ...

▸ RECENT CHANGES
  (2 hours ago) Added rate limiting middleware
  (1 day ago)   Refactored user service to Prisma

▸ ARCHITECTURE
  Request → auth middleware → rate limiter →
  route handler → service → model → Prisma → PostgreSQL

  [Context: 4,230 tokens of 8,000 budget]

$ ctxpkg status
  ✓ Project: my-api (TypeScript, Express + Prisma)
  ✓ Context size: 4,230 tokens / 8,000 budget
  ✓ Last pack: 2 minutes ago
  ✓ All manual sections present (architecture, conventions)
  → 2 files changed since last pack
```

---

*This PRD is a living document. Update as decisions are made and the product evolves.*
