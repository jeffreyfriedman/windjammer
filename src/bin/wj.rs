// wj - Unified Windjammer CLI
//
// This is the new unified CLI for Windjammer that provides a simpler,
// more cohesive development experience.
//
// Usage:
//   wj new <name>        Create a new project
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
    /// Create a new Windjammer project
    New {
        /// Project name
        #[arg(value_name = "NAME")]
        name: String,

        /// Project template (cli, web, lib, wasm)
        #[arg(short, long, default_value = "cli")]
        template: String,
    },

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

        /// Target platform (rust, javascript, wasm)
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto")]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,
    },

    /// Compile and run a Windjammer file
    Run {
        /// Path to .wj file
        #[arg(value_name = "FILE")]
        path: PathBuf,

        /// Arguments to pass to the program
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// Target platform (rust, javascript, wasm)
        #[arg(short, long, value_name = "TARGET", default_value = "rust")]
        target: String,

        /// Defer drop optimization mode (auto, always, never)
        #[arg(long, value_name = "MODE", default_value = "auto")]
        defer_drop: String,

        /// Defer drop threshold in bytes (default: 102400 = 100KB)
        #[arg(long, value_name = "BYTES")]
        defer_drop_threshold: Option<usize>,
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

    /// Add a dependency to wj.toml
    Add {
        /// Package name
        #[arg(value_name = "PACKAGE")]
        package: String,

        /// Package version
        #[arg(short, long)]
        version: Option<String>,

        /// Comma-separated list of features
        #[arg(short, long)]
        features: Option<String>,

        /// Local path dependency
        #[arg(short, long)]
        path: Option<String>,
    },

    /// Remove a dependency from wj.toml
    Remove {
        /// Package name
        #[arg(value_name = "PACKAGE")]
        package: String,
    },

    /// Update Windjammer to the latest version
    Update {
        /// Check for updates without installing
        #[arg(long)]
        check: bool,

        /// Force reinstall even if already up to date
        #[arg(long)]
        force: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::New { name, template } => {
            windjammer::cli::new::handle_new_command(&name, &template)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Commands::Build {
            path,
            output,
            release,
            target,
            defer_drop,
            defer_drop_threshold,
        } => {
            // TODO: Pass defer_drop config to compiler
            // For now, just ignore these flags - defer drop is always auto
            let _ = (defer_drop, defer_drop_threshold);
            windjammer::cli::build::execute(&path, output.as_deref(), release, &target)?;
        }
        Commands::Run {
            path,
            args,
            target,
            defer_drop,
            defer_drop_threshold,
        } => {
            // TODO: Pass defer_drop config to compiler
            // For now, just ignore these flags - defer drop is always auto
            let _ = (defer_drop, defer_drop_threshold);
            windjammer::cli::run::execute(&path, &args, &target)?;
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
        Commands::Add {
            package,
            version,
            features,
            path,
        } => {
            windjammer::cli::add::execute(
                &package,
                version.as_deref(),
                features.as_deref(),
                path.as_deref(),
            )?;
        }
        Commands::Remove { package } => {
            windjammer::cli::remove::execute(&package)?;
        }

        Commands::Update { check, force } => {
            windjammer::cli::update::execute(check, force)?;
        }
    }

    Ok(())
}
