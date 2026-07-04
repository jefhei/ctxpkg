mod cli;
mod clipboard;
mod config;
mod detect;
mod error;
mod packer;
mod scanner;

use anyhow::Result;
use std::path::PathBuf;

fn main() -> Result<()> {
    let cli = cli::Cli::parse();
    match cli.command {
        cli::Command::Init { path } => cmd_init(path),
        cli::Command::Pack {
            output,
            token_budget,
            format,
        } => cmd_pack(output, token_budget, format),
        cli::Command::Inject { token_budget } => cmd_inject(token_budget),
        cli::Command::Status { verbose } => cmd_status(verbose),
        cli::Command::Graft { pattern } => cmd_graft(pattern),
    }
}

fn cmd_init(path: Option<PathBuf>) -> Result<()> {
    let root = path.unwrap_or_else(|| std::env::current_dir().unwrap());
    println!("Initializing ctxpkg in: {}", root.display());
    // TODO: implement scanner, create .ctxpkg/ structure
    eprintln!("⚠ init not yet implemented");
    Ok(())
}

fn cmd_pack(
    _output: Option<PathBuf>,
    _token_budget: Option<usize>,
    _format: cli::OutputFormat,
) -> Result<()> {
    eprintln!("⚠ pack not yet implemented");
    Ok(())
}

fn cmd_inject(_token_budget: Option<usize>) -> Result<()> {
    eprintln!("⚠ inject not yet implemented");
    Ok(())
}

fn cmd_status(_verbose: bool) -> Result<()> {
    eprintln!("⚠ status not yet implemented");
    Ok(())
}

fn cmd_graft(_pattern: String) -> Result<()> {
    eprintln!("⚠ graft not yet implemented");
    Ok(())
}
