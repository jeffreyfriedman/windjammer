// wj - Unified Windjammer CLI
//
// This is the unified CLI for Windjammer. Parsing lives in `wj_cli_args`; dispatch in `wj_cli_cmds`.
//
// Usage:
//   wj new <name>        Create a new project
//   wj build <file>      Build Windjammer project
//   wj run <file>        Compile and execute
//   wj test              Run tests
//   wj fmt               Format code
//   wj lint              Run linter
//   wj check             Type check without building

mod wj_cli_args;
mod wj_cli_cmds;

use clap::Parser;
use wj_cli_args::Cli;

fn main() -> anyhow::Result<()> {
    // `wj --version` / `-V`: identity line (commit, date, dirty flag, profile, build date), before clap and
    // before any other work.
    if matches!(std::env::args().nth(1).as_deref(), Some("--version" | "-V")) && std::env::args().len() == 2 {
        println!("{}", windjammer::toolchain::identity::current().version_line());
        return Ok(());
    }
    let cli = Cli::parse();
    wj_cli_cmds::run(cli)
}
