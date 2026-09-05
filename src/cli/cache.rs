//! `wj cache` — shared Cargo target status, prune, and setup.

use anyhow::Result;
use colored::*;
use crate::cargo_cache::{
    self, format_size, prune, write_setup_scripts, PruneOptions, TargetKind,
    DEFAULT_INCREMENTAL_MAX_AGE_DAYS, DEFAULT_MIN_FREE_GIB,
};

pub fn execute_path(verify: bool) -> Result<()> {
    let kind = if verify {
        TargetKind::Verify
    } else {
        TargetKind::Shared
    };
    let dir = cargo_cache::target_dir(kind);
    println!("{}", dir.display());
    Ok(())
}

pub fn execute_status() -> Result<()> {
    let s = cargo_cache::status();
    println!("{}", "Windjammer Cargo cache".cyan().bold());
    println!("  root:    {}", s.root.display());
    println!(
        "  shared:  {} ({})",
        s.shared.display(),
        format_size(s.shared_bytes)
    );
    println!(
        "  verify:  {} ({})",
        s.verify.display(),
        format_size(s.verify_bytes)
    );
    match s.free_bytes {
        Some(b) => println!("  free:    {} on volume", format_size(b)),
        None => println!("  free:    (unknown)"),
    }
    if s.using_local_fallback {
        println!(
            "  {}",
            "note: using local ./target fallback (set WJ_CARGO_TARGET_ROOT or fix permissions)"
                .yellow()
        );
    }
    println!();
    println!(
        "  Low-disk prune threshold: {} GiB (override with WJ_DISK_FREE_MIN_GIB)",
        DEFAULT_MIN_FREE_GIB
    );
    println!(
        "  Incremental max age: {} days",
        DEFAULT_INCREMENTAL_MAX_AGE_DAYS
    );
    Ok(())
}

pub fn execute_prune(aggressive: bool) -> Result<()> {
    // Soft reclaim before reporting so long agent sessions stay under the threshold.
    let report = prune(PruneOptions {
        aggressive,
        auto_low_disk: true,
        ..PruneOptions::default()
    });

    if report.low_disk {
        println!(
            "{}",
            "Low free disk — applied aggressive incremental prune."
                .yellow()
                .bold()
        );
    }

    if report.freed_bytes == 0 && report.removed_paths.is_empty() {
        println!("{} Nothing to prune", "ok".green());
    } else {
        println!(
            "{} Freed approximately {}",
            "done".green().bold(),
            format_size(report.freed_bytes)
        );
        for msg in &report.messages {
            println!("  {}", msg);
        }
        if report.removed_paths.len() <= 20 {
            for p in &report.removed_paths {
                println!("  removed {}", p.display());
            }
        } else {
            println!("  removed {} paths", report.removed_paths.len());
        }
    }
    Ok(())
}

pub fn execute_setup() -> Result<()> {
    let (sh, ps1) = write_setup_scripts()?;
    println!("{}", "Wrote shell helpers:".cyan().bold());
    println!("  {}", sh.display());
    println!("  {}", ps1.display());
    println!();
    println!("macOS / Linux — add to your shell profile:");
    println!("  source {}", sh.display());
    println!();
    println!("Windows PowerShell — add to your profile:");
    println!("  . '{}'", ps1.display());
    println!();
    println!(
        "wj / wj-game set CARGO_TARGET_DIR automatically; sourcing is for bare `cargo` invocations."
    );
    println!("Opt out: WJ_USE_LOCAL_TARGET=1");
    Ok(())
}
