//! Each expectation is hand-written from the language's documented behaviour (exit statuses: docs/INTERPRETER.md;
//! sandbox grants: ADR-0021 section 5), not from a previous run of this crate.

use super::*;

fn go(src: &str) -> Outcome {
    run(src, &Options::default())
}

fn with(src: &str, f: impl FnOnce(&mut Options)) -> Outcome {
    let mut o = Options::default();
    f(&mut o);
    run(src, &o)
}

#[test]
fn hello_prints_and_exits_zero() {
    let o = go("fn main() {\n    println(\"hello, playground\")\n}\n");
    assert_eq!(o.stdout, "hello, playground\n", "{o:?}");
    assert_eq!(o.exit_code, 0, "{o:?}");
    assert!(o.diagnostics.is_empty(), "{o:?}");
}

#[test]
fn loops_and_structs_run() {
    let src = "struct P {\n    x: int,\n}\n\nfn main() {\n    let mut total = 0\n    for i in 0..5 {\n        total += i\n    }\n    let p = P { x: total }\n    println(\"${p.x}\")\n}\n";
    let o = go(src);
    assert_eq!((o.stdout.as_str(), o.exit_code), ("10\n", 0), "{o:?}");
}

#[test]
fn an_infinite_loop_runs_out_of_fuel() {
    let o = with(
        "fn main() {\n    let mut i = 0\n    loop {\n        i += 1\n    }\n}\n",
        |o| o.fuel = 100_000,
    );
    assert_eq!(o.exit_code, 124, "{o:?}");
    assert!(o.stderr.contains("fuel"), "{o:?}");
    assert!(o.steps >= 100_000, "{o:?}");
}

#[test]
fn output_is_bounded() {
    let o = with(
        "fn main() {\n    loop {\n        println(\"spam\")\n    }\n}\n",
        |o| o.output_bytes = 100,
    );
    assert_eq!(o.exit_code, 124, "{o:?}");
    assert!(o.stderr.contains("output"), "{o:?}");
}

#[test]
fn a_type_error_is_refused_with_the_compilers_diagnostic() {
    let o = go("fn main() {\n    let x: int = \"text\"\n    println(\"${x}\")\n}\n");
    assert_eq!(o.exit_code, 1, "{o:?}");
    assert_eq!(o.stdout, "", "the program must not run: {o:?}");
    let first = o.diagnostics.first().expect("a diagnostic");
    assert_eq!(first["severity"], "error", "{o:?}");
    let code = first["code"].as_str().unwrap();
    assert!(code.starts_with("WJ"), "{code}");
    assert!(
        o.stderr.contains(code),
        "the rendered diagnostic names its code: {o:?}"
    );
    assert_eq!(first["primary_span"]["line_start"], 2, "{first}");
}

#[test]
fn a_panic_exits_101_with_its_message() {
    let o = go("fn main() {\n    println(\"before\")\n    panic(\"boom\")\n}\n");
    assert_eq!(o.stdout, "before\n", "{o:?}");
    assert_eq!(o.exit_code, 101, "{o:?}");
    assert!(o.stderr.contains("boom"), "{o:?}");
}

#[test]
fn process_exit_sets_the_status() {
    let o = go("use std::process\n\nfn main() {\n    println(\"bye\")\n    process::exit(3)\n}\n");
    assert_eq!((o.stdout.as_str(), o.exit_code), ("bye\n", 3), "{o:?}");
}

#[test]
fn files_are_denied() {
    let o = go("use std::fs\n\nfn main() {\n    match fs::read_to_string(\"/etc/hosts\") {\n        Ok(t) => println(\"${t.len()}\"),\n        Err(e) => println(\"err ${e}\"),\n    }\n}\n");
    assert_eq!(o.exit_code, 126, "a denied capability stops the run: {o:?}");
    assert_eq!(o.stdout, "", "{o:?}");
    assert!(o.stderr.contains("fs.read"), "{o:?}");
}

#[test]
fn environment_is_denied() {
    let o = go("use std::env\n\nfn main() {\n    let home = env::var(\"HOME\").unwrap_or(\"none\")\n    println(home)\n}\n");
    assert_eq!(o.exit_code, 126, "{o:?}");
}

#[test]
fn time_and_random_are_a_function_of_the_seed() {
    let src = "use std::random\nuse std::time\n\nfn main() {\n    println(\"${time::now().timestamp()} ${random::range(0, 1000000)}\")\n}\n";
    let a = go(src);
    let b = go(src);
    assert_eq!(a.exit_code, 0, "{a:?}");
    assert_eq!(a.stdout, b.stdout, "same seed, same world");
    // the simulated world's clock starts at Unix time 1_700_000_000 (ADR-0021 E3)
    assert!(a.stdout.starts_with("1700000000 "), "{a:?}");
    let c = with(src, |o| o.seed = 7);
    assert_ne!(a.stdout, c.stdout, "another seed draws other numbers");
}

#[test]
fn json_carries_every_field() {
    let o = go("fn main() {\n    println(\"x\")\n}\n");
    let j: serde_json::Value = serde_json::from_str(&o.to_json()).unwrap();
    assert_eq!(j["stdout"], "x\n");
    assert_eq!(j["exit_code"], 0);
    assert!(
        j["diagnostics"].is_array() && j["stderr"].is_string() && j["steps"].is_u64(),
        "{j}"
    );
}
