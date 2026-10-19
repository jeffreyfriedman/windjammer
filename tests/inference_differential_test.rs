//! Differential / property harness for ownership, mutability, clone and width inference.
//!
//! Independent oracles (none of them is the code under test):
//!   1. `rustc` must accept the emitted Rust (borrow checker, move checker and type checker).
//!   2. The compiled program's stdout must equal the tree-walking interpreter's stdout
//!      (the interpreter has value semantics and no ownership inference).
//!   3. Each generated function has a usage template whose parameter modes follow from the rules in
//!      `docs/spec/inference.md`; the emitted signature must match.
//!
//! Programs come from a seeded generator (`WJ_INFER_SEEDS`, `WJ_INFER_SEED0`) and failures are
//! shrunk (drop calls, then functions) before being reported. Minimal repros of open divergences
//! live in `tests/inference_corpus/` and are listed in `.config/known-test-failures.txt`.

//! consumes: tests/inference_corpus/*.wj

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------------------------
// Seeded generator
// ---------------------------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
enum Ty {
    Vec,
    Bag,
    Cell,
    Str,
    Holder,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Ref,
    Mut,
    Owned,
}

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Mode::Ref => "&",
            Mode::Mut => "&mut",
            Mode::Owned => "owned",
        }
    }
}

/// A function template: signature, body, and the modes the spec requires for its parameters.
#[derive(Clone, Copy, Debug)]
struct Template {
    name: &'static str,
    params: &'static [(&'static str, Ty, Mode)],
    ret: Option<Ty>,
    body: &'static str,
}

const TEMPLATES: &[Template] = &[
    Template { name: "read_vec", params: &[("v", Ty::Vec, Mode::Ref)], ret: None, body: "v.len()" },
    Template { name: "read_bag", params: &[("b", Ty::Bag, Mode::Ref)], ret: None, body: "b.items.len()" },
    Template { name: "read_str", params: &[("s", Ty::Str, Mode::Ref)], ret: None, body: "s.len()" },
    Template { name: "mut_cell", params: &[("c", Ty::Cell, Mode::Mut)], ret: None, body: "c.n = c.n + 1\n    c.n" },
    Template { name: "push_bag", params: &[("b", Ty::Bag, Mode::Mut), ("s", Ty::Str, Mode::Owned)], ret: None, body: "b.items.push(s)\n    b.items.len()" },
    Template { name: "give_vec", params: &[("v", Ty::Vec, Mode::Owned)], ret: Some(Ty::Vec), body: "v" },
    Template { name: "give_bag", params: &[("b", Ty::Bag, Mode::Owned)], ret: Some(Ty::Bag), body: "b" },
    Template { name: "wrap_bag", params: &[("b", Ty::Bag, Mode::Owned), ("t", Ty::Str, Mode::Owned)], ret: Some(Ty::Holder), body: "Holder { bag: b, tag: t }" },
];

#[derive(Clone, Debug)]
struct Call {
    template: usize,
    /// Variable index per parameter (into the variable list of the matching type).
    args: Vec<usize>,
}

#[derive(Clone, Debug)]
struct Program {
    /// Variable declarations in order: (type, index within type).
    vars: Vec<Ty>,
    calls: Vec<Call>,
    /// Which templates are emitted as functions.
    templates: Vec<usize>,
}

fn var_name(ty: Ty, n: usize) -> String {
    let p = match ty {
        Ty::Vec => "v",
        Ty::Bag => "b",
        Ty::Cell => "c",
        Ty::Str => "s",
        Ty::Holder => "h",
    };
    format!("{p}{n}")
}

fn generate(seed: u64) -> Program {
    let mut rng = Rng(seed);
    let mut vars: Vec<Ty> = Vec::new();
    let mut counts = std::collections::HashMap::<Ty, usize>::new();
    let mut calls = Vec::new();
    let mut declare = |ty: Ty, vars: &mut Vec<Ty>| -> usize {
        let n = counts.entry(ty).or_insert(0);
        *n += 1;
        vars.push(ty);
        *n - 1
    };
    // Base values of each type exist up front so any call can be generated.
    for ty in [Ty::Vec, Ty::Bag, Ty::Cell, Ty::Str] {
        declare(ty, &mut vars);
    }
    let ncalls = 3 + rng.below(8);
    let mut pool: std::collections::HashMap<Ty, Vec<usize>> = std::collections::HashMap::new();
    for ty in [Ty::Vec, Ty::Bag, Ty::Cell, Ty::Str] {
        pool.insert(ty, vec![0]);
    }
    for _ in 0..ncalls {
        let t = rng.below(TEMPLATES.len());
        let tpl = &TEMPLATES[t];
        let mut args = Vec::new();
        for (_, ty, _) in tpl.params {
            let p = &pool[ty];
            args.push(p[rng.below(p.len())]);
        }
        if let Some(ret) = tpl.ret {
            let n = declare(ret, &mut vars);
            pool.entry(ret).or_default().push(n);
        }
        calls.push(Call { template: t, args });
    }
    let mut templates: Vec<usize> = calls.iter().map(|c| c.template).collect();
    templates.sort();
    templates.dedup();
    Program { vars, calls, templates }
}

fn ty_src(ty: Ty) -> &'static str {
    match ty {
        Ty::Vec => "Vec<int>",
        Ty::Bag => "Bag",
        Ty::Cell => "Cell",
        Ty::Str => "string",
        Ty::Holder => "Holder",
    }
}

fn render(p: &Program) -> String {
    let mut s = String::new();
    s.push_str("struct Cell { n: int, tag: string }\nstruct Bag { items: Vec<string>, name: string }\nstruct Holder { bag: Bag, tag: string }\n\n");
    for &t in &p.templates {
        let tpl = &TEMPLATES[t];
        let params: Vec<String> = tpl.params.iter().map(|(n, ty, _)| format!("{n}: {}", ty_src(*ty))).collect();
        let ret = match tpl.ret {
            Some(ty) => ty_src(ty).to_string(),
            None => "int".to_string(),
        };
        let ret = if tpl.name.starts_with("read_") || tpl.name == "mut_cell" || tpl.name == "push_bag" {
            "int".to_string()
        } else {
            ret
        };
        s.push_str(&format!("fn {}({}) -> {} {{\n    {}\n}}\n\n", tpl.name, params.join(", "), ret, tpl.body));
    }
    s.push_str("fn main() {\n");
    let mut seen = std::collections::HashMap::<Ty, usize>::new();
    // Declare base variables.
    let mut base_done: HashSet<Ty> = HashSet::new();
    for &ty in &p.vars {
        if matches!(ty, Ty::Holder) || base_done.contains(&ty) {
            continue;
        }
        base_done.insert(ty);
        let name = var_name(ty, 0);
        let init = match ty {
            Ty::Vec => "vec![1, 2, 3]".to_string(),
            Ty::Bag => "Bag { items: Vec::new(), name: \"bag\" }".to_string(),
            Ty::Cell => "Cell { n: 0, tag: \"cell\" }".to_string(),
            Ty::Str => "\"text\"".to_string(),
            Ty::Holder => unreachable!(),
        };
        let kw = if matches!(ty, Ty::Bag | Ty::Cell) { "let mut" } else { "let" };
        s.push_str(&format!("    {kw} {name} = {init}\n"));
        seen.insert(ty, 1);
    }
    let mut made = std::collections::HashMap::<Ty, usize>::new();
    let mut results = 0usize;
    for c in &p.calls {
        let tpl = &TEMPLATES[c.template];
        let args: Vec<String> = tpl.params.iter().zip(&c.args).map(|((_, ty, _), n)| var_name(*ty, *n)).collect();
        match tpl.ret {
            Some(rt) => {
                let n = made.entry(rt).or_insert(if matches!(rt, Ty::Holder) { 0 } else { 1 });
                let name = var_name(rt, *n);
                *n += 1;
                let kw = if matches!(rt, Ty::Bag) { "let mut" } else { "let" };
                s.push_str(&format!("    {kw} {name} = {}({})\n", tpl.name, args.join(", ")));
                let show = match rt {
                    Ty::Vec => format!("{name}.len()"),
                    Ty::Bag => format!("{name}.items.len()"),
                    Ty::Holder => format!("{name}.bag.items.len()"),
                    _ => name.clone(),
                };
                s.push_str(&format!("    println(\"${{{show}}}\")\n"));
            }
            None => {
                // Bound through a `let`: a call written directly inside `${}` hits the open
                // divergence D-NUM-INTERP-CALL (covered by tests/inference_corpus/interp_call_width.wj).
                results += 1;
                let name = format!("r{results}");
                s.push_str(&format!("    let {name} = {}({})\n", tpl.name, args.join(", ")));
                s.push_str(&format!("    println(\"${{{name}}}\")\n"));
            }
        }
    }
    // Observe final state of every base variable.
    s.push_str("    println(\"${v0.len()} ${b0.items.len()} ${c0.n} ${s0.len()}\")\n}\n");
    s
}

// ---------------------------------------------------------------------------------------------
// Running the oracles
// ---------------------------------------------------------------------------------------------

fn wj() -> &'static str {
    env!("CARGO_BIN_EXE_wj")
}

struct Out {
    ok: bool,
    stdout: String,
    stderr: String,
}

fn run(mut cmd: Command, limit: Duration) -> Out {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return Out { ok: false, stdout: String::new(), stderr: e.to_string() },
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() > limit => {
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break,
        }
    }
    match child.wait_with_output() {
        Ok(o) => Out {
            ok: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
        },
        Err(e) => Out { ok: false, stdout: String::new(), stderr: e.to_string() },
    }
}

#[derive(Debug)]
enum Failure {
    CompileWj(String),
    Rustc(String),
    Mismatch { interp: String, compiled: String },
    Signature(String),
    InterpFailed(String),
}

impl Failure {
    fn class(&self) -> &'static str {
        match self {
            Failure::CompileWj(_) => "wj-compile",
            Failure::Rustc(_) => "rustc-rejects",
            Failure::Mismatch { .. } => "output-differs",
            Failure::Signature(_) => "signature-mode",
            Failure::InterpFailed(_) => "interp-failed",
        }
    }
}

fn check_source(
    src: &str,
    dir: &Path,
    expect: &[(String, Vec<(String, Mode)>)],
    use_interpreter: bool,
) -> Result<(), Failure> {
    std::fs::create_dir_all(dir).unwrap();
    let wj_file = dir.join("prog.wj");
    std::fs::write(&wj_file, src).unwrap();
    let out_dir = dir.join("out");
    let _ = std::fs::remove_dir_all(&out_dir);
    let b = run(
        {
            let mut c = Command::new(wj());
            c.args(["build"]).arg(&wj_file).args(["--no-cargo", "--no-lint", "--quiet", "-o"]).arg(&out_dir);
            c
        },
        Duration::from_secs(120),
    );
    let rs = out_dir.join("prog.rs");
    if !b.ok || !rs.exists() {
        return Err(Failure::CompileWj(format!("{}{}", b.stdout, b.stderr)));
    }
    let rust = std::fs::read_to_string(&rs).unwrap();
    // Oracle 3: signature modes.
    if let Some(file) = windjammer::ir::explain::emitted::parse_emitted(&rust) {
        for (f, params) in expect {
            let Some(e) = file.fns.get(f) else { continue };
            for (pname, want) in params {
                let got = e.params.iter().find(|p| &p.name == pname).map(|p| p.mode.label());
                if got != Some(want.label()) {
                    return Err(Failure::Signature(format!(
                        "fn {f} param {pname}: spec says {}, emitted {:?}",
                        want.label(),
                        got
                    )));
                }
            }
        }
    }
    // Oracle 1: rustc.
    let bin = dir.join("prog_bin");
    let rc = run(
        {
            let mut c = Command::new("rustc");
            c.args(["--edition", "2021", "-A", "warnings", "-o"]).arg(&bin).arg(&rs);
            c
        },
        Duration::from_secs(180),
    );
    if !rc.ok {
        let first: String = rc.stderr.lines().filter(|l| l.starts_with("error")).take(3).collect::<Vec<_>>().join(" | ");
        return Err(Failure::Rustc(first));
    }
    // Oracle 2: interpreter vs compiled output.
    if !use_interpreter {
        return Ok(());
    }
    let compiled = run(Command::new(&bin), Duration::from_secs(20));
    let interp = run(
        {
            let mut c = Command::new(wj());
            c.args(["run", "--interpret"]).arg(&wj_file);
            c
        },
        Duration::from_secs(60),
    );
    if !interp.ok {
        return Err(Failure::InterpFailed(interp.stderr));
    }
    let interp_out: String = interp.stdout.lines().filter(|l| !l.starts_with("Interpreting ")).collect::<Vec<_>>().join("\n");
    let compiled_out = compiled.stdout.lines().collect::<Vec<_>>().join("\n");
    if interp_out.trim() != compiled_out.trim() {
        return Err(Failure::Mismatch { interp: interp_out, compiled: compiled_out });
    }
    Ok(())
}

fn expectations(p: &Program) -> Vec<(String, Vec<(String, Mode)>)> {
    p.templates
        .iter()
        .map(|&t| {
            let tpl = &TEMPLATES[t];
            (tpl.name.to_string(), tpl.params.iter().map(|(n, _, m)| (n.to_string(), *m)).collect())
        })
        .collect()
}

fn run_program(p: &Program, dir: &Path) -> Result<(), Failure> {
    check_source(&render(p), dir, &expectations(p), true)
}

/// Greedy shrinker: remove calls one at a time, then keep the smallest program with the same failure class.
fn shrink(p: &Program, class: &str, dir: &Path) -> Program {
    let mut cur = p.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for i in (0..cur.calls.len()).rev() {
            let mut cand = cur.clone();
            cand.calls.remove(i);
            // Variables produced by the removed call may be referenced later: skip such candidates.
            if !renders_consistently(&cand) {
                continue;
            }
            cand.templates = {
                let mut t: Vec<usize> = cand.calls.iter().map(|c| c.template).collect();
                t.sort();
                t.dedup();
                t
            };
            if let Err(f) = run_program(&cand, &dir.join("shrink")) {
                if f.class() == class {
                    cur = cand;
                    changed = true;
                    break;
                }
            }
        }
    }
    cur
}

/// A candidate is consistent if every variable a call reads was declared by an earlier call.
fn renders_consistently(p: &Program) -> bool {
    let mut made: std::collections::HashMap<Ty, usize> = std::collections::HashMap::new();
    for c in &p.calls {
        let tpl = &TEMPLATES[c.template];
        for ((_, ty, _), n) in tpl.params.iter().zip(&c.args) {
            let base = if matches!(ty, Ty::Holder) { 0 } else { 1 };
            if *n >= base && *n >= base + made.get(ty).copied().unwrap_or(0) {
                return false;
            }
        }
        if let Some(rt) = tpl.ret {
            *made.entry(rt).or_insert(0) += 1;
        }
    }
    true
}

fn work_dir(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("wj_infer_{}_{}", name, std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

fn env_u64(k: &str, d: u64) -> u64 {
    std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d)
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[test]
fn generator_is_deterministic_and_well_formed() {
    for seed in 0..50 {
        let a = render(&generate(seed));
        let b = render(&generate(seed));
        assert_eq!(a, b, "seed {seed} not deterministic");
        assert!(renders_consistently(&generate(seed)), "seed {seed} references an undeclared variable");
    }
    assert_ne!(render(&generate(1)), render(&generate(2)));
}

#[test]
fn shrinker_precondition_rejects_dangling_references() {
    // give_vec then read_vec on its result; removing the producer must be rejected.
    let p = Program {
        vars: vec![Ty::Vec, Ty::Vec],
        calls: vec![
            Call { template: 5, args: vec![0] },
            Call { template: 0, args: vec![1] },
        ],
        templates: vec![0, 5],
    };
    assert!(renders_consistently(&p));
    let mut q = p.clone();
    q.calls.remove(0);
    assert!(!renders_consistently(&q));
}

/// The property: generated programs compile under rustc, agree with the interpreter and
/// have the parameter modes the spec prescribes.
#[test]
fn generated_programs_satisfy_inference_oracles() {
    let n = env_u64("WJ_INFER_SEEDS", 6);
    let first = env_u64("WJ_INFER_SEED0", 1);
    let mut failures = Vec::new();
    for seed in first..first + n {
        let p = generate(seed);
        let dir = work_dir(&format!("s{seed}"));
        if let Err(f) = run_program(&p, &dir) {
            let small = shrink(&p, f.class(), &dir);
            failures.push(format!(
                "seed {seed}: {} ({:?})\n--- minimal program ---\n{}",
                f.class(),
                f,
                render(&small)
            ));
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(failures.is_empty(), "{} failing seed(s):\n{}", failures.len(), failures.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Corpus of minimal repros for open divergences (see docs/spec/inference-divergences.md).
// Each is expected to pass once its divergence is fixed; until then it is listed in
// .config/known-test-failures.txt.
// ---------------------------------------------------------------------------------------------

fn corpus_check(name: &str) {
    corpus_check_with(name, true)
}

/// Closure calls currently evaluate to `None` in the interpreter (separate stream), so the
/// closure repros are judged by rustc alone.
fn corpus_check_with(name: &str, use_interpreter: bool) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/inference_corpus").join(format!("{name}.wj"));
    let src = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    let dir = work_dir(name);
    let r = check_source(&src, &dir, &[], use_interpreter);
    let _ = std::fs::remove_dir_all(&dir);
    if let Err(f) = r {
        panic!("corpus `{name}` fails oracle {}: {:?}", f.class(), f);
    }
}

#[test]
fn corpus_for_accumulator_width_matches_element_width() {
    corpus_check("for_accumulator_width");
}

#[test]
fn corpus_string_literal_variable_moved_then_used() {
    corpus_check("string_literal_move_then_use");
}

#[test]
fn corpus_closure_len_arithmetic_width() {
    corpus_check_with("closure_len_width", false);
}

#[test]
fn corpus_closure_mutating_capture_is_mutable() {
    corpus_check_with("closure_mut_capture", false);
}

#[test]
fn corpus_explicit_ref_pin_is_not_double_borrowed() {
    corpus_check("pin_ref_vec");
}

#[test]
fn corpus_unsuffixed_integer_literal_defaults_to_int() {
    corpus_check("default_int_literal");
}

#[test]
fn corpus_call_inside_interpolation_constrains_literal_width() {
    corpus_check("interp_call_width");
}

#[test]
fn corpus_int_accumulator_over_int_fields_is_not_truncated_to_i32() {
    corpus_check("int_accumulator_field_width");
}
