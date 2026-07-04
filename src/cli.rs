use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ctxpkg", about = "Context Packages — project context for AI coding assistants")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Suppress non-essential output
    #[arg(global = true, short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Output in JSON format
    #[arg(global = true, long = "json")]
    pub json: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialize ctxpkg in a project
    Init {
        /// Project path (default: current directory)
        path: Option<std::path::PathBuf>,
    },
    /// Assemble a context package
    Pack {
        /// Output file (default: stdout)
        #[arg(short = 'o', long = "output")]
        output: Option<std::path::PathBuf>,

        /// Token budget (0 = unlimited, default: 8000)
        #[arg(short = 'b', long = "token-budget")]
        token_budget: Option<usize>,

        /// Output format
        #[arg(short = 'f', long = "format", default_value = "markdown")]
        format: OutputFormat,
    },
    /// Pack and copy to clipboard
    Inject {
        /// Token budget (0 = unlimited, default: 8000)
        #[arg(short = 'b', long = "token-budget")]
        token_budget: Option<usize>,
    },
    /// Show project context status
    Status {
        /// Show detailed per-section breakdown
        #[arg(short = 'v', long = "verbose")]
        verbose: bool,
    },
    /// Add a file or glob pattern to context
    Graft {
        /// File path or glob pattern to include
        pattern: String,
    },
}

#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Text,
    Markdown,
}

impl Cli {
    pub fn parse() -> Self {
        <Self as Parser>::parse()
    }
}
