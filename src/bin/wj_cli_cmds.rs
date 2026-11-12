//! Dispatch handlers for the `wj` binary (`cargo`-friendly shim crate splits parsing vs logic).

use crate::wj_cli_args::{CacheCommand, Cli, Commands, McpCommand};
use anyhow::Result;

/// Directory whose pin governs a build/run/test/check invocation.
fn pin_start(cmd: &Commands) -> Option<std::path::PathBuf> {
    let cwd = || std::env::current_dir().unwrap_or_default();
    let abs = |p: &std::path::Path| if p.is_absolute() { p.to_path_buf() } else { cwd().join(p) };
    match cmd {
        Commands::Build { path, .. } => Some(abs(path)),
        Commands::Run { path_and_args, .. } => Some(path_and_args.first().map(|p| abs(std::path::Path::new(p))).unwrap_or_else(cwd)),
        Commands::Test { path, .. } | Commands::Check { path, .. } => Some(path.as_deref().map(abs).unwrap_or_else(cwd)),
        _ => None,
    }
}

fn run_toolchain(cmd: &Commands) {
    use windjammer::toolchain;
    if let Some(start) = pin_start(cmd) {
        if let Err(msg) = toolchain::pin::enforce(&start) {
            eprintln!("{msg}");
            std::process::exit(1);
        }
    }
    if !matches!(cmd, Commands::Doctor { .. } | Commands::Version { .. } | Commands::InstallDev { .. }) {
        toolchain::stale::warn_if_stale();
        toolchain::behind::warn_if_behind();
    }
}

fn cmd_doctor(json: bool, expect_head: bool, strict: bool) -> Result<()> {
    use windjammer::toolchain::doctor;
    let path = windjammer::toolchain::env::get("PATH").unwrap_or_default();
    if expect_head {
        match doctor::expect_head(&path) {
            Ok(m) => println!("{m}"),
            Err(m) => {
                eprintln!("wj doctor --expect-head: {m}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    let report = doctor::collect(&path);
    if json {
        println!("{}", serde_json::to_string_pretty(&doctor::render_json(&report))?);
    } else {
        print!("{}", doctor::render_text(&report));
        let legacy = windjammer::metadata::legacy_sidecars::find(&std::env::current_dir().unwrap_or_default());
        if !legacy.is_empty() {
            println!(
                "note: {} legacy .wj.meta sidecar(s) next to sources are ignored and can be deleted: wj clean --sidecars",
                legacy.len()
            );
        }
        if let Some(note) = windjammer::cli::tmp_janitor::doctor_note() {
            println!("{note}");
        }
    }
    if !report.problems.is_empty() || (strict && !report.warnings.is_empty()) {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_install_dev(to: Option<std::path::PathBuf>, from: Option<std::path::PathBuf>, force: bool, dry_run: bool) -> Result<()> {
    use windjammer::toolchain::install;
    let src = match from {
        Some(f) => f,
        None => std::env::current_exe()?,
    };
    let dest = match to {
        Some(t) => t,
        None => install::default_dest().map_err(|e| anyhow::anyhow!(e))?,
    };
    match install::install(&src, &dest, force, dry_run) {
        Ok(i) if i.already_current => println!("{} already holds this build ({})", i.dest.display(), i.identity.version_line()),
        Ok(i) => {
            let verb = if dry_run { "would install" } else { "installed" };
            println!("{verb} {} to {}", i.identity.version_line(), i.dest.display());
            if let Some(b) = i.backup {
                println!("{} {}", if dry_run { "would back up the previous binary to" } else { "previous binary kept at" }, b.display());
            }
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
    Ok(())
}

/// Toolchain commands, plus the pin and staleness checks that precede every command. `Some` when handled.
fn toolchain_pre(cmd: &Commands) -> Option<Result<()>> {
    run_toolchain(cmd);
    match cmd {
        Commands::Version { json } => {
            let id = windjammer::toolchain::identity::current();
            Some(if *json { serde_json::to_string_pretty(&id.to_json()).map(|j| println!("{j}")).map_err(Into::into) } else { Ok(println!("{}", id.version_line())) })
        }
        Commands::Doctor { json, expect_head, strict } => Some(cmd_doctor(*json, *expect_head, *strict)),
        Commands::InstallDev { to, from, force, dry_run } => Some(cmd_install_dev(to.clone(), from.clone(), *force, *dry_run)),
        Commands::SelfInstall { dry_run } => Some(windjammer::cli::self_install::execute(*dry_run)),
        _ => None,
    }
}

pub fn run(cli: Cli) -> Result<()> {
    if let Some(done) = toolchain_pre(&cli.command) { return done }
    match cli.command {
        Commands::New { name, template } => {
            windjammer::cli::new::handle_new_command(&name, &template)
                .map_err(|e| anyhow::anyhow!("{}", e))?;
        }
        Commands::Build {
            path,
            output,
            release,
            unchecked_arithmetic,
            target,
            defer_drop,
            defer_drop_threshold,
            tree_shake,
            source_maps,
            check,
            raw_errors,
            fix,
            verbose,
            quiet,
            filter_file,
            filter_type,
            library,
            module_file,
            no_cargo,
            no_lint,
            no_generate_cargo_toml,
            metadata,
            json,
            ir_shadow_validate,
        } => {
            let _ = (defer_drop, defer_drop_threshold);
            windjammer::cli::build::execute(
                &path,
                output.as_deref(),
                release,
                &target,
                windjammer::cli::build::BuildOptions { tree_shake, source_maps, unchecked_arithmetic },
                check,
                raw_errors,
                fix,
                verbose,
                quiet,
                filter_file.as_deref(),
                filter_type.as_deref(),
                library,
                module_file,
                !no_cargo,
                !no_lint,
                no_generate_cargo_toml,
                &metadata,
                json,
                ir_shadow_validate,
            )?;
        }
        Commands::Run {
            path_and_args,
            bin,
            target,
            interpret,
            compiled,
            release,
            watch,
            verify, speculate,
            error_format,
            hot,
            sched_seed,
            hash_seed,
            engine,
            no_jit,
            jit,
            jit_loop_hot,
            jit_debug,
            jit_mutant,
            no_lint, error_format, no_locals,
            deterministic_time,
            allow_network,
            mock_http,
            record,
            replay,
            journal,
            crash_after,
            bind,
            limits,
            result_json,
            simulate,
            seed,
            faults,
            sandbox,
            allow,
            caps,
            defer_drop,
            defer_drop_threshold,
        } => {
            let _ = (defer_drop, defer_drop_threshold);
            let output = output_of(error_format.as_deref());
            use windjammer::cli::run_dev::{execute, Mode, RunOptions};
            let (path, args) = split_path_and_args(path_and_args);
            if let Some(code) = run_loop_mode(verify, watch, &target, path.as_deref(), bin.as_deref(), &args, (no_lint, output))? {
                std::process::exit(code);
            }
            let mode = run_mode(compiled, release, interpret || hot || record.is_some() || replay.is_some() || journal.is_some());
            let lint = windjammer::cli_execution::report_build_lints;
            let engine_resolved = engine_is_resolved(&engine)?;
            let jit_on = jit_setting(&jit, no_jit)?;
            let interp = windjammer::interpreter::runner::RunOptions { sched_seed, hash_seed, deterministic_time, allow_network, mock_http, engine_resolved, sandbox, allow, caps_mode: caps, record, replay, journal, crash_after, binds: bind, limits, result_json, durable_sites: Vec::new(), simulate: simulate.then_some(windjammer::interpreter::effect_port::sim::SimConfig { seed, faults }), no_jit: !jit_on, jit_loop_hot, jit_debug, jit_mutant, json_errors: error_format == "json", no_locals };
            let code = execute(&RunOptions { path, bin, args, mode, target, hot, interp, lint: Some(lint), no_lint, speculate, output });
            if code != 0 {
                std::process::exit(code);
            }
        }
        Commands::Eval { code } => windjammer::cli::eval::exit_on_failure(windjammer::cli::eval::run(&code)?),
        Commands::Mcp { command } => match command {
            McpCommand::Bind { server, tools, stdio, only, out } => {
                let text = match (tools, stdio) {
                    (Some(p), _) if p.as_os_str() == "-" => {
                        let mut t = String::new();
                        std::io::Read::read_to_string(&mut std::io::stdin(), &mut t)?;
                        t
                    }
                    (Some(p), _) => std::fs::read_to_string(&p).map_err(|e| anyhow::anyhow!("cannot read {}: {e}", p.display()))?,
                    (None, Some(cmd)) => windjammer::interpreter::mcp_host::list_tools(&cmd).map_err(anyhow::Error::msg)?,
                    (None, None) => anyhow::bail!("give the tool list with --tools FILE (or - for stdin), or ask a live server with --stdio COMMAND"),
                };
                let module = windjammer::cli::mcp_bind::generate_only(&server, &text, &only).map_err(anyhow::Error::msg)?;
                match out {
                    Some(p) => {
                        if let Some(dir) = p.parent().filter(|d| !d.as_os_str().is_empty()) {
                            std::fs::create_dir_all(dir)?;
                        }
                        std::fs::write(&p, module)?;
                    }
                    None => print!("{module}"),
                }
            }
        },
        Commands::Repl {} => run_repl()?,
        Commands::Test {
            path,
            filter,
            nocapture,
            parallel,
            json,
            library,
            module_file,
            output,
            use_build_dir,
            use_project_cargo,
            no_runtime_copy,
            copy_runtime,
            runtime_path,
            no_generate_cargo_toml,
            list,
            allow_empty,
            interpret,
            simulate,
            seed,
            seeds,
            faults,
            watch,
            doc,
            compiled,
        } => {
            if watch {
                let child_args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--watch").collect();
                let root = windjammer::cli::watch::watch_root(path.as_deref());
                std::process::exit(windjammer::cli::watch::watch(&root, child_args, None));
            }
            // ADR-0018: doctests run first (interpreter; `--compiled` also through Rust). Not in `--list`/`--json`.
            let doc_count = if list || json { 0 } else { run_doctests(path.as_deref(), filter.as_deref(), compiled)? };
            if doc {
                return Ok(());
            }
            let allow_empty = allow_empty || doc_count > 0;
            if interpret {
                let sim = sim_run(simulate, seeds, seed, faults)?;
                // ADR-0031: the crate host for the project's surfaced Rust crates (built once, cached)
                let here = path.clone().unwrap_or_else(|| std::path::PathBuf::from("."));
                let crates = if list { None } else { windjammer::cli::crate_host_build::host_for_path(&here, &mut |m| eprintln!("{m}")).map_err(|e| anyhow::anyhow!(e))? };
                return windjammer::test_runner::interpreted::run_interpreted(path.as_deref(), filter.as_deref(), list, allow_empty, json, sim, crates);
            }
            windjammer::run_tests_with_options(windjammer::TestRunOptions {
                path,
                filter,
                nocapture,
                parallel,
                json,
                library,
                module_file,
                output,
                use_build_dir,
                use_project_cargo,
                no_runtime_copy,
                copy_runtime,
                runtime_path,
                no_generate_cargo_toml,
                list,
                allow_empty,
            })?;
        }
        Commands::Reduce {
            path,
            oracle,
            output,
            jobs,
            seed,
            max_steps,
            time_budget,
            candidate_timeout,
            keep_names,
            wj_bin,
            verbose,
        } => {
            windjammer::cli::reduce::execute(windjammer::cli::reduce::Args {
                path,
                oracle,
                output,
                jobs,
                seed,
                max_steps,
                time_budget,
                candidate_timeout,
                keep_names,
                wj_bin,
                verbose,
            })?;
        }
        Commands::Fmt { check, rust, paths } => {
            windjammer::cli::fmt::execute(check, &paths, rust)?;
        }
        Commands::Fix { path, check, docs } => {
            windjammer::cli::fix::execute(&path, check, docs)?;
        }
        Commands::Lint { path, strict } => {
            windjammer::cli::lint::execute(&path, strict)?;
        }
        Commands::Check { path, json, watch, fix, error_format } => {
            windjammer::cli::check::execute(path.as_deref(), json, watch, fix, output_of(error_format.as_deref()).format)?
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
        Commands::Stats { clear, verbose } => {
            if clear {
                let mut stats = windjammer::error_statistics::load_or_create_stats();
                stats.clear();
                windjammer::error_statistics::save_stats(&stats)?;
                println!("Statistics cleared!");
            } else {
                let stats = windjammer::error_statistics::load_or_create_stats();
                println!("{}", if verbose { stats.format_verbose() } else { stats.format() });
            }
        }
        Commands::Docs { query, limit, json, output, format } => {
            if query.is_empty() {
                cmd_generate_docs(&output, &format)?
            } else {
                windjammer::agent_index::docs_search::run_cli(&query.join(" "), limit, json)
            }
        }
        Commands::ExplainOwnership { file, function, json } => windjammer::cli::explain_ownership::execute(&file, function.as_deref(), json)?,
        Commands::ExplainEffects { path, function, json } => windjammer::cli::explain_effects::execute(&path, function.as_deref(), json)?,
        Commands::Caps { action, path, json } => windjammer::cli::caps::execute(&action, path.as_deref().unwrap_or_else(|| std::path::Path::new(".")), json)?,
        Commands::RustSurface { crate_name, functions, all_pub: _, write, path } => windjammer::cli::rust_surface::execute(
            path.as_deref().unwrap_or_else(|| std::path::Path::new(".")), &crate_name, &functions, write)?,
        Commands::Explain { code } => cmd_explain(&code)?,
        Commands::AgentIndex { output } => {
            windjammer::agent_index::generate_agent_index(&output)?;
        }
        Commands::ShaderCompile { input, output } => cmd_shader_compile(&input, output.as_deref())?,
        Commands::Errors { path, code, severity } => windjammer::cli::errors::execute(path.as_deref().unwrap_or_else(|| std::path::Path::new(".")), code.as_deref(), severity.as_deref())?,
        Commands::ValidateWjsl { path } => cmd_validate_wjsl(&path)?,
        Commands::Eject(a) => windjammer::cli::eject::execute(&a.path, &a.output, &a.target, a.library, a.module_file)?,
        Commands::Clean { all, sidecars, tmp, older_than_hours, yes } => {
            windjammer::cli::clean::execute(all, sidecars, tmp, older_than_hours, yes)?;
        }
        Commands::Cache { action } => match action {
            CacheCommand::Path { verify } => windjammer::cli::cache::execute_path(verify)?,
            CacheCommand::Status => windjammer::cli::cache::execute_status()?,
            CacheCommand::Prune { aggressive, force } => windjammer::cli::cache::execute_prune(aggressive, force)?,
            CacheCommand::Warm => windjammer::cli::cache::execute_warm()?,
            CacheCommand::Setup => windjammer::cli::cache::execute_setup()?,
        },
        Commands::Version { .. } | Commands::Doctor { .. } | Commands::InstallDev { .. } | Commands::SelfInstall { .. } => {}
        Commands::Plugin(plugin_args) => cmd_plugin(&plugin_args)?,
    }

    Ok(())
}

/// `wj <plugin> <args>`: run an external plugin and exit with its status.
fn cmd_plugin(plugin_args: &[String]) -> Result<()> {
    if plugin_args.is_empty() {
        anyhow::bail!("Plugin name required. Usage: wj <plugin> <args>");
    }
    // an unknown subcommand is most often a `wj` older than the one the docs describe: say which build this is
    let exit_code = windjammer::plugin::execute_plugin(&plugin_args[0], &plugin_args[1..]).map_err(|e| {
        let build = windjammer::toolchain::behind::current_behind_text()
            .unwrap_or_else(|| format!("(this is {})", windjammer::toolchain::identity::current().version_line()));
        e.context(build)
    })?;
    std::process::exit(exit_code);
}

fn cmd_generate_docs(output: &std::path::Path, format: &str) -> Result<()> {
    use std::fs;

    println!("Generating error documentation...");

    let catalog = windjammer::error_catalog::ErrorCatalog::new();

    fs::create_dir_all(output)?;

    match format {
        "html" => {
            let html = catalog.generate_html();
            let path = output.join("index.html");
            fs::write(&path, html)?;
            println!("✓ Generated HTML documentation: {}", path.display());
        }
        "markdown" | "md" => {
            let md = catalog.generate_markdown();
            let path = output.join("errors.md");
            fs::write(&path, md)?;
            println!("✓ Generated Markdown documentation: {}", path.display());
        }
        "json" => {
            let path = output.join("errors.json");
            catalog.save_json(&path)?;
            println!("✓ Generated JSON catalog: {}", path.display());
        }
        _ => {
            anyhow::bail!(
                "Unknown format: {}. Use 'html', 'markdown', or 'json'",
                format
            );
        }
    }

    println!("\n📚 Error documentation generated successfully!");
    println!("   {} errors documented", catalog.errors.len());
    println!("   {} categories", catalog.categories.len());

    Ok(())
}

fn cmd_explain(code: &str) -> Result<()> {
    if let Some(text) = windjammer::diagnostics::codes::explain(&code.to_uppercase()) {
        println!("{text}");
        return Ok(());
    }
    let registry = windjammer::error_codes::get_registry();
    let catalog = windjammer::error_catalog::ErrorCatalog::new();
    let lint = windjammer::linter::codes::LINT_CODES.iter().find(|l| l.code == code);

    if let Some(wj_code) = registry.get(code) {
        println!("{}", registry.format_explanation(&wj_code.code));
    } else if let Some(wj_code) = registry.map_rust_code(code) {
        println!("{}", registry.format_explanation(&wj_code.code));
    } else if let Some(e) = catalog.get_error(code) {
        println!("{} - {}\n\n{}", e.code, e.title, e.explanation);
        for s in &e.solutions {
            println!("  - {s}");
        }
        for ex in &e.examples {
            println!("\n  bad:  {}\n  good: {}", ex.bad_code, ex.good_code);
        }
    } else if let Some(l) = lint {
        println!("{} - {} (default level: {})\n\n{}", l.code, l.name, l.default_level, l.summary);
        println!("\nRun `wj lint <path>` to list occurrences; `wj check --json` reports them with spans.");
    } else {
        use colored::*;
        eprintln!("{}", format!("Error code '{}' not found", code).red());
        eprintln!("\n{}", "Available codes:".yellow());

        let mut codes: Vec<_> = registry.all_codes();
        codes.sort_by_key(|c| c.code.as_str());

        for error_code in codes {
            eprintln!("  {} - {}", error_code.code.cyan(), error_code.title);
        }
        for l in windjammer::linter::codes::LINT_CODES {
            eprintln!("  {} - {}", l.code.cyan(), l.name);
        }

        eprintln!("\n{}", "Usage:".yellow());
        eprintln!("  wj explain WJ0001");
        eprintln!("  wj explain W0002   (lint code)");
        eprintln!("  wj explain E0425   (Rust error code)");
        anyhow::bail!("unknown error code '{code}'");
    }

    Ok(())
}

fn cmd_shader_compile(input: &std::path::Path, output: Option<&std::path::Path>) -> Result<()> {
    let source = std::fs::read_to_string(input)
        .map_err(|e| anyhow::anyhow!("Failed to read {}: {}", input.display(), e))?;
    let wgsl = windjammer::wjsl::compile_legacy_shader(&source)?;
    match output {
        Some(path) => {
            std::fs::write(path, &wgsl)?;
            println!("Compiled {} -> {}", input.display(), path.display());
        }
        None => {
            print!("{}", wgsl);
        }
    }
    Ok(())
}

fn cmd_validate_wjsl(path: &std::path::Path) -> Result<()> {
    use colored::*;
    use std::path::Path;

    println!("{}", "Validating WJSL shaders...".bold());

    let mut errors = 0u32;
    let mut validated = 0u32;

    fn find_wjsl_files(dir: &Path, files: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    find_wjsl_files(&p, files);
                } else if p.extension().is_some_and(|e| e == "wjsl") {
                    files.push(p);
                }
            }
        }
    }

    if !path.exists() {
        anyhow::bail!("path not found: {}", path.display());
    }
    let mut wjsl_files = Vec::new();
    if path.is_file() {
        wjsl_files.push(path.to_path_buf());
    } else {
        find_wjsl_files(path, &mut wjsl_files);
    }
    wjsl_files.sort();

    for file in &wjsl_files {
        let source = match std::fs::read_to_string(file) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  {} {}: {}", "✗".red(), file.display(), e);
                errors += 1;
                continue;
            }
        };

        let base_dir = file.parent().unwrap_or(Path::new("."));
        match windjammer::wjsl::transpile_wjsl_with_includes(&source, base_dir) {
            Ok(_) => {
                validated += 1;
            }
            Err(e) => {
                eprintln!("  {} {}: {}", "✗".red(), file.display(), e);
                errors += 1;
            }
        }
    }

    if errors > 0 {
        eprintln!(
            "\n{} {} shader(s) validated, {} error(s)",
            "WJSL validation failed:".red().bold(),
            validated,
            errors
        );
        std::process::exit(1);
    } else {
        println!(
            "  {} {} shader(s) validated successfully",
            "✓".green(),
            validated
        );
    }

    Ok(())
}

/// Run the Windjammerscript REPL (state persists between inputs).
fn run_repl() -> Result<()> {
    // ADR-0031: calls into the surfaced Rust crates of the project around the working directory
    let crates = windjammer::cli::crate_host_build::host_for_repl(std::path::Path::new("."), &mut |m| eprintln!("{m}")).unwrap_or_else(|e| {
        eprintln!("wj: Rust crate calls are not available in this REPL: {e}");
        None
    });
    windjammer::interpreter::repl::run_repl_with(crates);
    Ok(())
}

/// Run the doctests under `path`; exits non-zero on a failure. Returns how many ran.
fn run_doctests(path: Option<&std::path::Path>, filter: Option<&str>, compiled: bool) -> Result<usize> {
    let root = path.unwrap_or_else(|| std::path::Path::new("."));
    let exe = std::env::current_exe()?;
    let results = windjammer::doctest::run::run_doctests(root, filter, compiled.then_some(exe.as_path()));
    if results.is_empty() {
        return Ok(0);
    }
    if windjammer::doctest::run::report(&results) > 0 {
        std::process::exit(1);
    }
    Ok(results.len())
}


/// `--jit on|off` (env `WJ_JIT`) and `--no-jit`: may hot integer functions run as machine code?
fn jit_setting(jit: &str, no_jit: bool) -> anyhow::Result<bool> {
    match jit {
        "on" | "1" | "true" => Ok(!no_jit),
        "off" | "0" | "false" => Ok(false),
        other => anyhow::bail!("unknown WJ_JIT / --jit value {other:?}: use on or off (or 1 / 0)"),
    }
}

/// `--seeds 1..200` (or a single number): an inclusive range of simulation seeds.
fn parse_seed_range(text: &str) -> anyhow::Result<std::ops::RangeInclusive<u64>> {
    let bad = || anyhow::anyhow!("--seeds takes A..B (for example 1..200), found {text:?}");
    let (a, b) = match text.split_once("..") {
        Some((a, b)) => (a.trim().parse::<u64>().map_err(|_| bad())?, b.trim().trim_start_matches('=').parse::<u64>().map_err(|_| bad())?),
        None => {
            let n = text.trim().parse::<u64>().map_err(|_| bad())?;
            (n, n)
        }
    };
    if a > b {
        return Err(bad());
    }
    Ok(a..=b)
}

/// `wj test --simulate` options: the seeds to try and the fault rate.
fn sim_run(simulate: bool, seeds: Option<String>, seed: Option<u64>, faults: f64) -> anyhow::Result<Option<windjammer::interpreter::runner::SimRun>> {
    if !simulate {
        return Ok(None);
    }
    let seeds = match (seeds, seed) {
        (Some(r), _) => parse_seed_range(&r)?,
        (None, s) => {
            let s = s.unwrap_or(1);
            s..=s
        }
    };
    Ok(Some(windjammer::interpreter::runner::SimRun { seeds, faults }))
}

fn run_mode(compiled: bool, release: bool, interpret: bool) -> windjammer::cli::run_dev::Mode {
    use windjammer::cli::run_dev::Mode;
    if compiled || release {
        Mode::Compiled { release }
    } else if interpret {
        Mode::Interpret
    } else {
        Mode::Auto
    }
}

fn engine_is_resolved(engine: &str) -> anyhow::Result<bool> {
    match engine {
        "resolved" => Ok(true),
        "walker" => Ok(false),
        other => anyhow::bail!("unknown --engine {other:?}: use walker or resolved"),
    }
}

/// `wj run [flags] PATH [ARGS]...`: the first value is the path, the rest belongs to the program (a leading `--`
/// separator, as older invocations wrote it, is dropped).
/// `wj run --verify` (interpret, build, compare: ADR-0033) and `wj run --watch` (restart on change) take over the whole
/// run; `None` for an ordinary run.
/// Diagnostic format (`--error-format`, else `WJ_ERROR_FORMAT`) and build progress (`WJ_PROGRESS=1`/`0`, else only on a
/// terminal) for `wj check` and `wj run` (ADR-0037). The driver is the one place that reads these variables.
fn output_of(flag: Option<&str>) -> windjammer::cli::run::Output {
    use windjammer::toolchain::env;
    let format = windjammer::diagnostics::render::Format::parse(flag.or(env::get("WJ_ERROR_FORMAT").as_deref()));
    let progress = env::get("WJ_PROGRESS").map(|v| v == "1");
    windjammer::cli::run::Output::for_stderr(format, progress)
}

fn run_loop_mode(verify: bool, watch: bool, target: &str, path: Option<&std::path::Path>, bin: Option<&str>, args: &[String], (no_lint, output): (bool, windjammer::cli::run::Output)) -> Result<Option<i32>> {
    if verify {
        if target != "rust" {
            anyhow::bail!("`--verify` compares with the Rust build; it does not take `--target {target}`");
        }
        return Ok(Some(windjammer::cli::verify::execute(path, bin, args, no_lint, output)));
    }
    if watch {
        let child_args: Vec<String> = std::env::args().skip(1).filter(|a| a != "--watch").collect();
        let root = windjammer::cli::watch::watch_root(path);
        return Ok(Some(windjammer::cli::watch::watch(&root, child_args, None)));
    }
    Ok(None)
}

fn split_path_and_args(mut rest: Vec<String>) -> (Option<std::path::PathBuf>, Vec<String>) {
    if rest.is_empty() {
        return (None, rest);
    }
    let path = std::path::PathBuf::from(rest.remove(0));
    if rest.first().is_some_and(|a| a == "--") {
        rest.remove(0);
    }
    (Some(path), rest)
}
