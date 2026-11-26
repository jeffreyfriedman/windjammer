#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! The implicit integer conversion table (`src/type_inference/int_conversion_table.rs`) against rustc.
//!
//! Every pair of {i8..i128, u8..u128, isize, usize}: the table's class must equal the class computed from
//! the types' ranges, and rustc itself must agree: for each pair one program per class is compiled and run
//! (`TryFrom` on MIN and MAX), so the table cannot drift from what the integer types actually are.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;
use windjammer::type_inference::int_conversion_table::{conversion, Conversion, ALL_WIDTHS};

/// Independent oracle: (min, max) as (i128 magnitude flags) is not enough for u128, so compare by bits/sign.
fn expected_lossless(from: &str, to: &str) -> bool {
    let info = |n: &str| (n.starts_with('i'), if n.ends_with("size") { 64 } else { n[1..].parse::<u32>().unwrap() });
    let ((fs, fb), (ts, tb)) = (info(from), info(to));
    match (fs, ts) {
        (true, true) | (false, false) => tb >= fb,
        (false, true) => tb > fb,
        (true, false) => false,
    }
}

/// One rustc program asserting, for every listed pair, `lossless` (both MIN and MAX convert) or its negation.
fn rustc_run(label: &str, pairs: &[(&str, &str)], lossless: bool) {
    let mut src = String::from("fn main() {\n");
    for (f, t) in pairs {
        let ok = format!("{t}::try_from({f}::MIN).is_ok() && {t}::try_from({f}::MAX).is_ok()");
        let line = if lossless { format!("    assert!({ok}, \"{f}->{t}\");\n") } else { format!("    assert!(!({ok}), \"{f}->{t}\");\n") };
        src.push_str(&line);
    }
    src.push_str("}\n");
    let s = Scratch::new(&format!("convtable-{label}"));
    let rs = s.path().join("t.rs");
    std::fs::write(&rs, src).unwrap();
    let bin = s.path().join("t");
    let c = Command::new("rustc").args(["--edition", "2021", "-A", "warnings", "-o"]).arg(&bin).arg(&rs).output().unwrap();
    assert!(c.status.success(), "{label}: {}", String::from_utf8_lossy(&c.stderr));
    let r = Command::new(&bin).output().unwrap();
    assert!(r.status.success(), "{label}: {}", String::from_utf8_lossy(&r.stderr));
}

#[test]
fn table_matches_rustc_for_every_pair() {
    let (mut widen, mut lossy, mut error) = (vec![], vec![], vec![]);
    for f in ALL_WIDTHS {
        for t in ALL_WIDTHS {
            let (fname, tname) = (f.rust_name(), t.rust_name());
            let class = conversion(f, t);
            assert_eq!(class == Conversion::Same, f == t, "{fname}->{tname}");
            if f == t {
                continue;
            }
            match class {
                Conversion::Widen => {
                    assert!(expected_lossless(fname, tname), "{fname}->{tname} listed Widen but is lossy");
                    widen.push((fname, tname));
                }
                Conversion::LossyAs => {
                    assert!(!expected_lossless(fname, tname), "{fname}->{tname} listed LossyAs but is lossless");
                    lossy.push((fname, tname));
                }
                Conversion::Error => {
                    assert!(!expected_lossless(fname, tname), "{fname}->{tname}: a lossless pair must be implicit");
                    error.push((fname, tname));
                }
                Conversion::Same => unreachable!(),
            }
        }
    }
    assert_eq!(widen.len() + lossy.len() + error.len(), 12 * 11);
    rustc_run("widen", &widen, true);
    rustc_run("lossy", &lossy, false);
    rustc_run("error", &error, false);
}
