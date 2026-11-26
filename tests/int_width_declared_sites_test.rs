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

//! The declared width wins at every declared-width site (docs/design/int-conversion-table.md).
//! Each case builds (`wj build --no-cargo` + `rustc`), runs the compiled program, runs the same program
//! interpreted, and requires identical output: (A) a `usize` length into any declared width, (B) an integer
//! literal into an f32 slot, (C) a literal into a `u128` slot, (D) a literal above `u64::MAX`.

#[path = "common/scratch.rs"]
mod scratch;
use scratch::Scratch;
use std::process::Command;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// (compiled stdout, interpreted stdout); panics with the diagnostics when either side fails.
fn both(name: &str, src: &str) -> (String, String) {
    let s = Scratch::new(&format!("declw-{name}"));
    let dir = s.path().to_path_buf();
    let wj = dir.join("main.wj");
    std::fs::write(&wj, src).unwrap();
    let out = dir.join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&wj).current_dir(&dir).output().unwrap();
    assert!(b.status.success(), "{name}: wj build: {}{}", String::from_utf8_lossy(&b.stdout), String::from_utf8_lossy(&b.stderr));
    let bin = dir.join("main_bin");
    let r = Command::new("rustc").args(["--edition", "2021", "-A", "warnings", "-o"]).arg(&bin).arg(out.join("main.rs")).output().unwrap();
    assert!(r.status.success(), "{name}: rustc: {}", String::from_utf8_lossy(&r.stderr));
    let c = Command::new(&bin).output().unwrap();
    assert!(c.status.success(), "{name}: compiled run: {}", String::from_utf8_lossy(&c.stderr));
    let i = Command::new(WJ).args(["run", "--interpret", "--no-lint"]).arg(&wj).output().unwrap();
    assert!(i.status.success(), "{name}: interpreted run: {}{}", String::from_utf8_lossy(&i.stdout), String::from_utf8_lossy(&i.stderr));
    (String::from_utf8_lossy(&c.stdout).into(), String::from_utf8_lossy(&i.stdout).into())
}

fn agree(name: &str, src: &str, expected: &str) {
    let (compiled, interpreted) = both(name, src);
    assert_eq!(compiled, expected, "{name}: compiled output");
    assert_eq!(interpreted, expected, "{name}: interpreted output");
}

#[test]
fn a_usize_length_converts_at_every_declared_width_site() {
    agree(
        "len",
        "struct Rec {\n    count: u64,\n}\nfn takes(n: u64) -> u64 {\n    n\n}\nfn count_of(v: Vec<int>) -> u64 {\n    v.len()\n}\nfn main() {\n    let v: Vec<int> = [1, 2, 3]\n    let size: int = v.len()\n    let small: i32 = v.len()\n    let wide: u64 = v.len()\n    let huge: u128 = v.len()\n    println(\"${size} ${small} ${wide} ${huge}\")\n    println(\"${takes(v.len())} ${count_of(v)}\")\n    let r = Rec { count: v.len() }\n    println(\"${r.count}\")\n}\n",
        "3 3 3 3\n3 3\n3\n",
    );
}

#[test]
fn b_an_integer_literal_into_an_f32_slot_is_a_float_literal() {
    agree(
        "f32lit",
        "fn main() {\n    let mut w: f32 = 1.5\n    w = 3\n    println(\"${w}\")\n    let mut r: f32 = 0.0\n    r = 16777217\n    println(\"${r}\")\n}\n",
        "3\n16777216\n",
    );
}

#[test]
fn c_and_d_wide_literals_take_the_declared_128_bit_type() {
    agree(
        "wide",
        "fn main() {\n    let h: u128 = 340282366920938463463374607431768211455\n    println(\"${h}\")\n    let m: u128 = 18446744073709551615\n    println(\"${m}\")\n    let i: i128 = 170141183460469231731687303715884105727\n    println(\"${i}\")\n    let n: i128 = -170141183460469231731687303715884105728\n    println(\"${n}\")\n    let big: u128 = 9223372036854775809\n    println(\"${big}\")\n}\n",
        "340282366920938463463374607431768211455\n18446744073709551615\n170141183460469231731687303715884105727\n-170141183460469231731687303715884105728\n9223372036854775809\n",
    );
}

#[test]
fn d_a_literal_above_u64_max_without_a_128_bit_slot_is_an_error() {
    let s = Scratch::new("declw-noctx");
    let wj = s.path().join("main.wj");
    std::fs::write(&wj, "fn main() {\n    let a = 18446744073709551616\n    println(\"${a}\")\n}\n").unwrap();
    let o = Command::new(WJ).arg("check").arg(&wj).output().unwrap();
    assert!(!o.status.success());
    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    assert!(text.contains("does not fit in 64 bits"), "{text}");
}
