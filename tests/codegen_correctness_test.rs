//! Rust-codegen correctness gates: compiled Rust must agree with the interpreter and with an
//! independent expectation.
//!
//! Two layers:
//! 1. `tests/codegen_correctness_corpus/*.wj`: minimal repros of confirmed codegen bugs. Each carries
//!    `// expect: line` header lines (the independent oracle, hand-derived). The program is run through
//!    `wj run --interpret` and through `wj build --no-cargo` + `rustc`; both must print the expectation.
//! 2. Seeded generated programs per bug class (`gen_*`): the same two executions are compared. A
//!    failing program is shrunk by deleting self-contained body units until the failure is minimal,
//!    and the shrunk source is printed.
//!
//! `WJ_CODEGEN_GATE_SEEDS=n` sets how many seeds each class runs (default 3).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

#[derive(Debug, PartialEq, Eq, Clone)]
struct Run {
    stdout: String,
    exit: i32,
}

fn interpret(src: &str) -> Result<Run, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let out = Command::new(WJ).args(["run", "--interpret"]).arg(&f).output().unwrap();
    if out.stdout.is_empty() && !out.status.success() {
        return Err(format!("interpreter failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(Run { stdout: String::from_utf8_lossy(&out.stdout).to_string(), exit: out.status.code().unwrap_or(-1) })
}

fn compiled(src: &str) -> Result<Run, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let wj = Command::new(WJ)
        .args(["build", "--target", "rust", "--no-cargo"])
        .arg(&f)
        .current_dir(dir.path())
        .output()
        .unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}", String::from_utf8_lossy(&wj.stderr)));
    }
    let rs = dir.path().join("build/test.rs");
    let bin = dir.path().join("bin");
    let rustc = Command::new("rustc").arg(&rs).arg("-o").arg(&bin).args(["--edition", "2021"]).output().unwrap();
    if !rustc.status.success() {
        let generated = fs::read_to_string(&rs).unwrap_or_default();
        return Err(format!("rustc failed: {}\n--- generated\n{}", String::from_utf8_lossy(&rustc.stderr), generated));
    }
    let run = Command::new(&bin).output().unwrap();
    Ok(Run { stdout: String::from_utf8_lossy(&run.stdout).to_string(), exit: run.status.code().unwrap_or(-1) })
}

/// Like `compiled`, but through the full `wj build` (cargo) so the program links `windjammer_runtime`.
fn compiled_with_runtime(src: &str) -> Result<Run, String> {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("prog.wj");
    fs::write(&f, src).unwrap();
    let wj = Command::new(WJ).args(["build"]).arg(&f).current_dir(dir.path()).env("WJ_AGENT_CHILD", "1").output().unwrap();
    if !wj.status.success() {
        return Err(format!("wj build failed: {}{}", String::from_utf8_lossy(&wj.stdout), String::from_utf8_lossy(&wj.stderr)));
    }
    let bin = dir.path().join("build/target/debug/prog");
    let run = Command::new(&bin).output().map_err(|e| format!("cannot run {}: {e}", bin.display()))?;
    Ok(Run { stdout: String::from_utf8_lossy(&run.stdout).to_string(), exit: run.status.code().unwrap_or(-1) })
}

/// `Ok(())` when interpreter and compiled Rust agree (and match `expect` when given).
fn check(src: &str, expect: Option<&str>) -> Result<(), String> {
    let rust = compiled(src)?;
    let interp = interpret(src)?;
    if let Some(e) = expect {
        if interp.stdout != e {
            return Err(format!("interpreter output differs from expectation\n--- got\n{}--- want\n{}", interp.stdout, e));
        }
    }
    if rust != interp {
        return Err(format!(
            "compiled Rust differs from the interpreter\n--- interpreter (exit {})\n{}--- rust (exit {})\n{}",
            interp.exit, interp.stdout, rust.exit, rust.stdout
        ));
    }
    Ok(())
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/codegen_correctness_corpus")
}

fn corpus_case(name: &str) {
    let path = corpus_dir().join(format!("{name}.wj"));
    let src = fs::read_to_string(&path).unwrap();
    let mut expect = String::new();
    for line in src.lines() {
        if let Some(r) = line.strip_prefix("// expect: ") {
            expect.push_str(r);
            expect.push('\n');
        }
    }
    assert!(!expect.is_empty(), "{name}: needs `// expect:` lines");
    // `// interp-gap: reason` marks a program the interpreter evaluates wrongly (reason in the file);
    // the compiled Rust must still print the hand-derived expectation.
    if src.lines().any(|l| l.starts_with("// interp-gap:")) {
        let build = if src.lines().any(|l| l.starts_with("// needs-runtime:")) { compiled_with_runtime } else { compiled };
        let rust = build(&src).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(rust.stdout, expect, "{name}: compiled output differs from the expectation");
        return;
    }
    if let Err(e) = check(&src, Some(&expect)) {
        panic!("{name}: {e}");
    }
}

macro_rules! corpus_tests {
    ($($name:ident),* $(,)?) => {
        $( #[test] fn $name() { corpus_case(stringify!($name)); } )*
    };
}

corpus_tests!(
    shadow_param_scalar_read_by_while,
    shadow_param_scalar_mutated,
    shadow_param_vec_mutated,
    shadow_param_nested_scopes,
    shadow_param_nested_blocks,
    shadow_param_closure_param,
    closure_capture_mutation,
    closure_to_fn_param,
    let_else,
    map_index_and_default,
    string_concat_to_string,
    inline_mod_use,
    manual_partial_eq,
    impl_trait_for_primitive,
    unwrap_or_ref_literal,
    self_mut_through_loop_var_fields,
    option_param_consumed,
    closure_return_fn,
    interpolated_widths,
    direct_iteration_adapters,
    self_mut_inference,
    std_idioms,
    literal_default_through_adapters,
    adapter_closure_param_depth,
    borrowed_item_equals_owned_value,
    loop_return_in_else_branch,
    text_slice_into_owned_string,
    generic_element_read_clone,
    loop_mut_writes_through_element,
    shadowed_loop_var_typed_later,
    field_iterated_inside_outer_loop,
    loop_mut_shadowed_after_write,
    element_read_then_collection_changed,
    caller_param_name_suffix_of_field_arg,
    element_read_borrowed_across_container_write,
    for_over_mut_param_then_push,
    rebound_name_other_type_in_sibling_scope,
    self_field_arg_into_mut_self_method,
    self_field_arg_into_mut_self_method_returned,
    assigned_value_read_after_return_arm_in_let,
    recursive_read_only_param_through_optional_field,
    trait_param_mutated_by_impl,
    char_closures_take_the_declared_item,
    borrowed_copy_key_into_map_lookup,
    enum_payload_copy_some_from_borrowed_scrutinee,
    issue_183_float_arg_arithmetic,
    issue_184_contains_key_borrowed_key,
    issue_185_trait_impl_keeps_declared_receiver,
    issue_186_while_move_of_text_argument,
    issue_187_vec_literal_tail_of_function,
    issue_188_indexed_row_binding_agrees_with_initializer,
    issue_189_reassigned_vector_is_moved,
    borrowed_key_by_operand_type,
    field_passed_to_owned_formal_keeps_borrowed_receiver,
    builder_reads_fields_after_storing_one,
    mut_param_is_own_copy,
    str_literal_loop_var_pushed_into_owned_vec,
    split_of_line_collected_into_owned_vec,
    mut_param_passed_to_reader_and_mut_callee,
    mut_param_read_before_write,
    lines_loop_var_pushed_in_branch,
    str_slice_pushed_into_owned_vec,
    option_field_unwrap_or_behind_borrow,
    sequence_predicates_borrow_their_receiver,
    str_array_element_in_owned_branch,
    borrowed_text_alias_reassigned_owned,
    owned_text_ordered_against_borrowed_text,
    nested_closure_typed_param,
    if_value_in_let_not_cast_to_return_width,
    int_element_compared_with_len_and_used_as_index,
    struct_field_read_moves_literal_to_i32,
    assert_in_match_let_value_takes_field_width,
    sequence_adapters_take_usize_counts,
    collect_after_map_item_type,
    substring_tail_value,
    text_slice_in_every_position,
    callable_struct_fields,
    match_or_pattern_return_arm_not_if_let_tail,
    strip_prefix_default_is_owned_text,
    split_last_default_is_owned_text,
    dyn_fn_signature,
    slice_bounds_from_find_index,
    int_cast_operand_not_promoted_to_float,
    list_literal_local_into_array_field,
    bytes_extended_from_slices_are_u8,
    text_slice_range_bounds_from_int,
    text_and_list_slices_pushed_into_owned_vecs,
    list_literal_of_if_text_into_owned_vec,
    match_value_of_int_and_len_used_as_index,
    cloned_vec_keeps_declared_element_width,
    u64_constant_at_two_to_the_63,
    name_rebound_from_int_to_byte_list,
    empty_list_assigned_to_typed_local,
    extend_with_copied_slice,
    string_as_bytes_to_vec_formal,
    vec_macro_tuple_string_slots,
    bitwise_vs_comparison_precedence,
    slice_to_vec_extend_argument,
    collect_owned_from_unknown_receiver,
    list_adapter_owned_option_return,
    integer_rotate_amount_is_u32,
    associated_type_binding_in_bound,

    bitwise_literal_takes_operand_width,
    float_byte_conversions,
    float_literal_compare_takes_float_width,
    generic_constructor_declared_argument_width,
    generic_self_argument_not_cloned,
    if_else_statement_unit_and_value_tails,
    if_else_statement_value_tail,
    if_value_owned_and_borrowed_text,
    index_by_borrowed_loop_var,
    loop_index_over_match_bound_list_is_int,
    list_literal_in_returned_tuple_slot,
    method_call_on_imported_constant,
    operator_impl_infers_output,
    operator_impl_selected_by_rhs_type,
    operator_traits_as_supertraits_imported,
    thread_handles,
    option_vec_some_literal_field,
    let_bound_closure_literal_argument,
    list_literal_takes_expected_vec_type,
    condition_const_before_block_is_not_struct_literal,
    index_cast_applies_to_whole_modulo_expression,
    for_var_shadows_struct_field,
    field_and_free_fn_same_name,
    option_str_unwrap_or_owned_default,
    local_string_list_contains_text_arg,
    if_arm_param_vs_substring,
    moved_into_tuple_then_read_in_list_literal,
    field_moved_then_whole_value_used,
    string_if_let_moved_then_read,
    match_value_arm_with_diverging_else,
    map_get_default_by_declared_copy_value,
    clone_of_split_piece_into_owned_list,
    if_let_variant_of_indexed_element,
    option_field_unwrap_or_then_whole_value,
    match_call_result_bound_twice_in_call,
    indexed_field_write_in_loop_over_field,
    list_literal_contains_text_arg,
    if_value_of_struct_params_as_owned_arg,
    self_mutation_through_call_cycle,
    self_mutation_through_three_cycle,
    param_mutation_through_call_cycle,
    push_borrowed_text_into_vec_in_struct,
    push_borrowed_text_into_vec_returned,
    push_borrowed_text_into_map_value_vec,
    explicit_float_int_conversions,
);

/// Every corpus `.wj` file is registered in `corpus_tests!` exactly once, and every registered name has a file:
/// an unregistered case would silently never run.
#[test]
fn every_corpus_case_is_registered_and_runs() {
    let src = include_str!("codegen_correctness_test.rs");
    let start = src.find("corpus_tests!(\n").expect("macro invocation") + "corpus_tests!(\n".len();
    let end = start + src[start..].find("\n);").expect("macro end");
    let mut registered: Vec<String> =
        src[start..end].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
    registered.sort();
    let mut files: Vec<String> = fs::read_dir(corpus_dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.strip_suffix(".wj").map(|s| s.to_string())
        })
        .collect();
    files.sort();
    let mut problems = Vec::new();
    for f in &files {
        if !registered.contains(f) {
            problems.push(format!("corpus case `{f}.wj` is not registered in corpus_tests! (it never runs)"));
        }
    }
    for r in &registered {
        if !files.contains(r) {
            problems.push(format!("corpus_tests! registers `{r}` but tests/codegen_correctness_corpus/{r}.wj does not exist"));
        }
    }
    for w in registered.windows(2) {
        if w[0] == w[1] {
            problems.push(format!("`{}` is registered more than once", w[0]));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}


// ------------------------------------------------------------------ generated programs

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

/// A program is a fixed prelude (items) and self-contained body units; deleting a unit keeps it valid.
#[derive(Clone)]
struct Prog {
    prelude: String,
    units: Vec<String>,
}

impl Prog {
    fn source(&self) -> String {
        let mut s = self.prelude.clone();
        s.push_str("fn main() {\n");
        for u in &self.units {
            s.push_str(u);
        }
        s.push_str("}\n");
        s
    }
}

fn seed_count() -> u64 {
    std::env::var("WJ_CODEGEN_GATE_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(3)
}

/// Delete units while the failure persists; return the minimal program and its error.
fn shrink(mut p: Prog, mut err: String) -> (Prog, String) {
    let mut i = 0;
    while i < p.units.len() {
        if p.units.len() == 1 {
            break;
        }
        let mut q = p.clone();
        q.units.remove(i);
        match check(&q.source(), None) {
            Err(e) => {
                p = q;
                err = e;
            }
            Ok(()) => i += 1,
        }
    }
    (p, err)
}

fn run_class(class: &str, generate: fn(&mut Rng) -> Prog) {
    for seed in 1..=seed_count() {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) ^ class.len() as u64);
        let p = generate(&mut rng);
        if let Err(e) = check(&p.source(), None) {
            let (min, err) = shrink(p, e);
            panic!("class {class}, seed {seed}: {err}\n=== shrunk program\n{}", min.source());
        }
    }
}

fn unit_count(rng: &mut Rng) -> u64 {
    rng.range(2, 4)
}

fn gen_closure_capture(rng: &mut Rng) -> Prog {
    let mut units = Vec::new();
    for i in 0..unit_count(rng) {
        let a = rng.range(0, 9);
        let (m, k) = (rng.range(2, 5), rng.range(1, 7));
        let calls = rng.range(1, 4);
        let body = match rng.range(0, 2) {
            0 => format!("c{i} += {k}"),
            1 => format!("c{i} = (c{i} * {m} + {k}) % 1000"),
            _ => format!("c{i} = c{i} + {k}"),
        };
        let mut u = format!("    let mut c{i} = {a}\n    let mut f{i} = || {{ {body} }}\n");
        for _ in 0..calls {
            u.push_str(&format!("    f{i}()\n"));
        }
        u.push_str(&format!("    println(\"${{c{i}}}\")\n"));
        units.push(u);
    }
    for i in 0..unit_count(rng) {
        let (b, m, x) = (rng.range(1, 9), rng.range(2, 5), rng.range(0, 9));
        units.push(format!(
            "    let base{i}: int = {b}\n    let g{i} = |x| x * {m} + base{i}\n    println(\"${{apply(g{i}, {x})}}\")\n"
        ));
    }
    Prog { prelude: "fn apply(f: fn(int) -> int, x: int) -> int {\n    f(x)\n}\n".into(), units }
}

fn gen_let_else(rng: &mut Rng) -> Prog {
    let mut units = Vec::new();
    for i in 0..unit_count(rng) + 1 {
        let arg = if rng.range(0, 1) == 0 { format!("Some({})", rng.range(0, 20)) } else { "None".to_string() };
        units.push(format!("    println(\"${{le{i}({arg})}}\")\n"));
    }
    let mut prelude = String::new();
    for i in 0..units.len() {
        let (mul, add, dflt) = (rng.range(1, 6), rng.range(0, 9), rng.range(0, 9));
        prelude.push_str(&format!(
            "fn le{i}(o: Option<int>) -> int {{\n    let Some(v) = o else {{ return {dflt} }}\n    v * {mul} + {add}\n}}\n"
        ));
    }
    Prog { prelude, units }
}

fn gen_map(rng: &mut Rng) -> Prog {
    let mut units = Vec::new();
    for i in 0..unit_count(rng) {
        let keys = ["a", "bb", "c3", "dd"];
        let mut u = format!("    let mut m{i}: HashMap<string, int> = HashMap::new()\n");
        for k in keys.iter().take(rng.range(1, 4) as usize) {
            u.push_str(&format!("    m{i}.insert(\"{k}\", {})\n", rng.range(0, 99)));
        }
        let probe = keys[rng.range(0, 3) as usize];
        let dflt = rng.range(0, 9);
        u.push_str(&format!("    let k{i} = \"{probe}\"\n"));
        u.push_str(&format!("    println(\"${{m{i}.contains_key(k{i})}}\")\n"));
        u.push_str(&format!("    let d{i} = m{i}.get(&k{i}).copied().unwrap_or({dflt})\n"));
        u.push_str(&format!("    println(\"${{d{i}}}\")\n"));
        if rng.range(0, 1) == 0 {
            u.push_str(&format!("    if m{i}.contains_key(k{i}) {{\n        println(\"${{m{i}[k{i}]}}\")\n    }}\n"));
        }
        units.push(u);
    }
    Prog { prelude: "use std::collections::HashMap\n".into(), units }
}

fn gen_concat(rng: &mut Rng) -> Prog {
    let mut units = Vec::new();
    for i in 0..unit_count(rng) + 1 {
        let (a, b) = (rng.range(0, 500), rng.range(0, 500));
        let lit = ["x", "n=", "id:", "v"][rng.range(0, 3) as usize];
        units.push(match rng.range(0, 2) {
            0 => format!("    let a{i}: int = {a}\n    let s{i} = \"{lit}\" + a{i}.to_string()\n    println(\"${{s{i}}}\")\n"),
            1 => format!(
                "    let a{i}: int = {a}\n    let b{i}: int = {b}\n    let s{i} = \"{lit}\" + a{i}.to_string() + \"-\" + b{i}.to_string()\n    println(\"${{s{i}}}\")\n"
            ),
            _ => format!("    let a{i}: int = {a}\n    let s{i} = a{i}.to_string() + \"{lit}\"\n    println(\"${{s{i}}}\")\n"),
        });
    }
    Prog { prelude: String::new(), units }
}

fn gen_impls(rng: &mut Rng) -> Prog {
    let mut prelude = String::from("trait Scale { fn scale(self) -> int }\nimpl Scale for int {\n");
    prelude.push_str(&format!("    fn scale(self) -> int {{ self * {} }}\n}}\n", rng.range(2, 9)));
    prelude.push_str("struct P { a: int, b: int }\nimpl PartialEq for P {\n    fn eq(self, other: P) -> bool { self.a == other.a }\n}\n");
    prelude.push_str("enum Tag { A, B }\nimpl PartialEq for Tag {\n    fn eq(self, other: Tag) -> bool { true }\n}\n");
    let mut units = Vec::new();
    for i in 0..unit_count(rng) {
        let (a, b, c) = (rng.range(0, 5), rng.range(0, 99), rng.range(0, 5));
        units.push(format!(
            "    let p{i} = P {{ a: {a}, b: {b} }}\n    let q{i} = P {{ a: {c}, b: {} }}\n    println(\"${{p{i} == q{i}}}\")\n",
            rng.range(0, 99)
        ));
        let n = rng.range(0, 30);
        units.push(format!("    let n{i}: int = {n}\n    println(\"${{n{i}.scale()}}\")\n"));
    }
    Prog { prelude, units }
}

fn gen_modules(rng: &mut Rng) -> Prog {
    let (m1, k1) = (rng.range(2, 6), rng.range(0, 9));
    let prelude = format!(
        "mod util {{\n    pub fn mix(x: int) -> int {{ x * {m1} + {k1} }}\n    pub fn bump(x: int) -> int {{ x + 1 }}\n}}\nuse util::mix\nuse util::bump\n"
    );
    let mut units = Vec::new();
    for i in 0..unit_count(rng) + 1 {
        let x = rng.range(0, 50);
        units.push(format!("    let x{i}: int = {x}\n    println(\"${{mix(bump(x{i}))}}\")\n"));
    }
    Prog { prelude, units }
}

macro_rules! gate {
    ($name:ident, $gen:ident) => {
        #[test]
        fn $name() {
            run_class(stringify!($name), $gen);
        }
    };
}

gate!(gate_closure_capture_class, gen_closure_capture);
gate!(gate_let_else_class, gen_let_else);
gate!(gate_map_lookup_class, gen_map);
gate!(gate_string_concat_class, gen_concat);
gate!(gate_trait_impl_class, gen_impls);
gate!(gate_inline_module_class, gen_modules);

// ------------------------------------------------------------------ harness self-test

#[test]
fn shrinker_reduces_to_the_failing_unit() {
    // A program whose only bad unit is the second one: `shrink` must drop the good ones.
    let good = |i: u32| format!("    let ok{i}: int = {i}\n    println(\"${{ok{i}}}\")\n");
    let bad = "    let r: int = undefined_symbol_here\n    println(\"${r}\")\n".to_string();
    let p = Prog { prelude: String::new(), units: vec![good(1), bad, good(3), good(4)] };
    let err = check(&p.source(), None).expect_err("a program with an unknown symbol must fail");
    let (min, _) = shrink(p, err);
    assert_eq!(min.units.len(), 1, "shrinker kept {} units", min.units.len());
    assert!(min.units[0].contains("undefined_symbol_here"));
}

// ------------------------------------------------------------------ library module layout

/// Every source directory with a `mod.wj` must get a `mod.rs` in a `--library --module-file` build,
/// including directories whose `mod.wj` is their only file, and the result must compile as a crate.
#[test]
fn library_build_emits_mod_rs_for_every_mod_wj_directory() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    let files: [(&str, &str); 4] = [
        ("mod.wj", "pub mod a\npub mod c\n"),
        ("a/mod.wj", "pub mod b\npub fn fa() -> int { 1 }\n"),
        ("a/b/mod.wj", "pub fn fb() -> int { 2 }\n"),
        ("c/mod.wj", "pub fn fc() -> int { 3 }\n"),
    ];
    for (rel, body) in files {
        let p = src.join(rel);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, body).unwrap();
    }
    let out = dir.path().join("gen");
    let o = Command::new(WJ)
        .args(["build"])
        .arg(src.join("mod.wj"))
        .args(["--output"])
        .arg(&out)
        .args(["--library", "--no-cargo", "--module-file"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    for rel in ["a", "a/b", "c"] {
        assert!(out.join(rel).join("mod.rs").exists(), "missing gen/{rel}/mod.rs");
    }
    assert!(out.join("lib.rs").exists(), "missing gen/lib.rs");
    let rustc = Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2021", "--emit", "metadata", "-o"])
        .arg(dir.path().join("out.rmeta"))
        .arg(out.join("lib.rs"))
        .output()
        .unwrap();
    assert!(rustc.status.success(), "generated library does not compile:\n{}", String::from_utf8_lossy(&rustc.stderr));
}

// ------------------------------------------------------------------ threads (shape gate)

/// Thread programs need the runtime crate (and the interpreter has no threads), so this gate inspects the
/// generated Rust: handles are pushed by move, spawn closures return their tail value, and a parameter
/// moved into a spawned closure is an owned formal that callers pass by value.
#[test]
fn thread_codegen_moves_handles_and_receivers() {
    let src = r#"use std::thread
use std::sync::mpsc
fn apply(stage: string, rx: Receiver<int>) -> Receiver<int> {
    let (tx2, rx2) = mpsc::channel()
    thread::spawn(move || {
        loop {
            match rx.recv() {
                Ok(v) => { tx2.send(v + 1) }
                Err(_) => { break }
            }
        }
    })
    rx2
}
fn main() {
    let mut hs: Vec<JoinHandle<int>> = []
    let mut i: int = 0
    while i < 2 {
        let h = thread::spawn(move || { 5 })
        hs.push(h)
        i += 1
    }
    let mut j: int = 0
    while j < 2 {
        let k = j
        hs.push(thread::spawn(move || {
            let s = k * 10
            s
        }))
        j += 1
    }
    for h in hs {
        println("${h.join().unwrap_or(0)}")
    }
    let (tx, rx) = mpsc::channel()
    let out = apply("a", rx)
    tx.send(1)
}
"#;
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("t.wj");
    fs::write(&f, src).unwrap();
    let o = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(dir.path()).output().unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let rs = fs::read_to_string(dir.path().join("build/t.rs")).unwrap();
    assert!(!rs.contains("h.clone()"), "JoinHandle must be moved, not cloned:\n{rs}");
    assert!(!rs.contains("let _ = s;"), "spawn closure lost its tail value:\n{rs}");
    assert!(rs.contains("rx: Receiver<i64>") && !rs.contains("&Receiver"), "Receiver moved into a thread must be owned:\n{rs}");
    assert!(rs.contains("apply(\"a\", rx)"), "call must move the receiver:\n{rs}");
    assert!(!rs.contains("(3_i32 as i32)") && rs.contains("while i < 2_i64"), "annotated counter width:\n{rs}");
}

/// `wj build` of `src`: the generated Rust and the compiler's stderr.
fn build_rust(src: &str) -> (String, String) {
    let dir = TempDir::new().unwrap();
    let f = dir.path().join("test.wj");
    fs::write(&f, src).unwrap();
    fs::create_dir_all(dir.path().join("build")).unwrap();
    let out = Command::new(WJ).args(["build", "--target", "rust", "--no-cargo"]).arg(&f).current_dir(dir.path()).output().unwrap();
    let rs = fs::read_to_string(dir.path().join("build/test.rs")).unwrap_or_default();
    (rs, String::from_utf8_lossy(&out.stderr).to_string() + &String::from_utf8_lossy(&out.stdout))
}

#[test]
fn issue_185_reports_w0016_and_keeps_the_receiver() {
    let src = fs::read_to_string(corpus_dir().join("issue_185_trait_impl_keeps_declared_receiver.wj")).unwrap();
    let (_, diag) = build_rust(&src);
    let out = Command::new(WJ).args(["check"]).arg(corpus_dir().join("issue_185_trait_impl_keeps_declared_receiver.wj")).output().unwrap();
    let all = diag + &String::from_utf8_lossy(&out.stderr) + &String::from_utf8_lossy(&out.stdout);
    assert!(all.contains("W0016"), "the undeclared trait receiver is reported:\n{all}");
}

#[test]
fn issue_189_reassigned_vector_is_not_cloned() {
    let src = fs::read_to_string(corpus_dir().join("issue_189_reassigned_vector_is_moved.wj")).unwrap();
    let (rs, _) = build_rust(&src);
    assert!(rs.contains("append_u32_le"), "{rs}");
    assert!(!rs.contains(".clone()"), "`bin = append_u32_le(bin, i)` moves the vector:\n{rs}");
}
