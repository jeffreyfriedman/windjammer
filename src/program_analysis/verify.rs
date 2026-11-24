//! Verify mode (G2): a reused product equals its recomputation. Equality is on a canonical dump: the pretty `Debug` text
//! with its lines sorted, so two equal hash maps iterating in different orders still compare equal.

use super::keys::AnalysisKey;
use super::Analysis;
use crate::ir::numeric_bridge::UnifiedNumericInference;

fn canonical(value: &impl std::fmt::Debug) -> Vec<String> {
    let mut lines: Vec<String> = format!("{value:#?}").lines().map(str::to_string).collect();
    lines.sort();
    lines
}

fn dump(a: &Analysis) -> Vec<String> {
    match &a.result {
        Ok(ok) => canonical(&(&ok.functions, &ok.registry, &ok.trait_methods)),
        Err(e) => vec![format!("error: {e}")],
    }
}

/// A mismatch ends the process: the consumers run stages under `catch_unwind` (a stage that panics is "no answer"), so a
/// panic here would be swallowed and the run would carry on with a wrong product. Exit code 86.
pub fn mismatch(what: &str) -> ! {
    eprintln!("program_analysis: {what}");
    std::process::exit(86)
}

pub fn analysis(key: &AnalysisKey, fresh: &Analysis, reused: &Analysis) {
    let (a, b) = (dump(fresh), dump(reused));
    if a != b {
        let first = |x: &[String], y: &[String]| {
            x.iter()
                .filter(|l| !y.contains(l))
                .take(6)
                .cloned()
                .collect::<Vec<_>>()
                .join("\n  ")
        };
        mismatch(&format!("the reused {} differs from its recomputation\nonly recomputed:\n  {}\nonly reused:\n  {}", key.describe(), first(&a, &b), first(&b, &a)));
    }
}

pub fn numeric(
    file: &super::FileKey,
    fresh: &UnifiedNumericInference,
    reused: &UnifiedNumericInference,
) {
    if fresh.errors != reused.errors {
        mismatch(&format!(
            "the reused numeric inference of {} differs from its recomputation",
            file.name()
        ));
    }
}
