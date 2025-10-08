// wj - Unified Windjammer CLI
//
// This is the new unified CLI for Windjammer that provides a simpler,
// more cohesive development experience.
//
// Usage:
//   wj build <file>      Build Windjammer project
//   wj run <file>        Compile and execute
//   wj test              Run tests
//   wj fmt               Format code
//   wj lint              Run linter
//   wj check             Type check without building

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "wj")]
#[command(about = "Windjammer - A simple language that transpiles to Rust", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Build a Windjammer project
    Build {
        /// Path to .wj file or directory
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Output directory (default: ./build)
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,

        /// Build in release mode
        #[arg(short, long)]
        release: bool,
    },

    /// Compile and run a Windjammer file
    Run {
        /// Path to .wj file
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Run tests
    Test {
        /// Test name filter
        #[arg(value_name = "FILTER")]
        filter: Option<String>,
    },

    /// Format Windjammer code
    Fmt {
        /// Check formatting without applying changes
        #[arg(long)]
        check: bool,
    },

    /// Run linter (clippy)
    Lint {
        /// Automatically fix warnings
        #[arg(long)]
        fix: bool,
    },

    /// Type check without building
    Check,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build {
            path,
            output,
            release,
        } => {
            windjammer::cli::build::execute(&path, output.as_deref(), release)?;
        }
        Commands::Run { path, args } => {
            windjammer::cli::run::execute(&path, &args)?;
        }
        Commands::Test { filter } => {
            windjammer::cli::test::execute(filter.as_deref())?;
        }
        Commands::Fmt { check } => {
            windjammer::cli::fmt::execute(check)?;
        }
        Commands::Lint { fix } => {
            windjammer::cli::lint::execute(fix)?;
        }
        Commands::Check => {
            windjammer::cli::check::execute()?;
        }
    }

    Ok(())
}

