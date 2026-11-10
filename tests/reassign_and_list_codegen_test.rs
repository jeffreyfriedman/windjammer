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
//! Whole-value reassignment of `&mut` formals, return-position list literals, owned-text bindings of
//! borrowed pieces, and `unwrap_or` on map lookups: each program's generated Rust must type-check
//! (the independent oracle is `rustc --emit metadata`) and, where the output is observable, keep its meaning.

use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

const WJ: &str = env!("CARGO_BIN_EXE_wj");

/// Builds `src` to Rust (no cargo) and returns (generated Rust, rustc stderr, rustc ok).
fn generate_and_typecheck(src: &str) -> (String, String, bool) {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("prog.wj");
    std::fs::write(&input, src).unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "-o"]).arg(&out).arg(&input).output().unwrap();
    assert!(b.status.success(), "wj build failed:\n{}", String::from_utf8_lossy(&b.stderr));
    let rs = out.join("prog.rs");
    let generated = std::fs::read_to_string(&rs).unwrap();
    let r = Command::new("rustc")
        .args(["--crate-type", "bin", "--edition", "2021", "--emit", "metadata", "-A", "warnings", "--out-dir"])
        .arg(dir.path())
        .arg(Path::new(&rs))
        .output()
        .unwrap();
    (generated, String::from_utf8_lossy(&r.stderr).into_owned(), r.status.success())
}

fn typechecks(src: &str) -> String {
    let (generated, err, ok) = generate_and_typecheck(src);
    assert!(ok, "generated Rust does not type-check:\n{err}\n--- generated ---\n{generated}");
    generated
}

// ---- whole-value reassignment of a `&mut` formal ----

#[test]
fn reassigned_vec_formal_is_written_through() {
    let g = typechecks(
        "fn reset(v: Vec<int>) {\n    v = Vec::new()\n    v.push(1)\n}\nfn main() {\n    let mut v = Vec::new()\n    reset(v)\n    println(\"${v.len()}\")\n}\n",
    );
    assert!(g.contains("*v = "), "{g}");
}

#[test]
fn reassigned_int_formal_inside_if_is_written_through() {
    let g = typechecks(
        "fn zero(slot: int, flag: bool) {\n    if flag {\n        slot = 0\n    }\n}\nfn main() {\n    let mut n = 5\n    zero(n, true)\n    println(\"${n}\")\n}\n",
    );
    assert!(g.contains("*slot = "), "{g}");
}

#[test]
fn reassigned_struct_formal_is_written_through() {
    typechecks(
        "struct P { x: int }\nfn replace(p: P) {\n    p = P { x: 9 }\n}\nfn main() {\n    let mut p = P { x: 1 }\n    replace(p)\n    println(\"${p.x}\")\n}\n",
    );
}

#[test]
fn reassigned_string_formal_is_a_mut_borrow_not_a_str() {
    let g = typechecks(
        "fn rs(s: string) {\n    s = \"x\"\n}\nfn main() {\n    let mut s = \"a\"\n    rs(s)\n    println(\"${s}\")\n}\n",
    );
    assert!(g.contains("s: &mut String"), "{g}");
}

// ---- list literals in return position are lists ----

#[test]
fn list_of_struct_literals_as_vec_return_tail() {
    typechecks(
        "pub struct Account {\n    pub code: string,\n}\n\nfn list_accounts(slug: string) -> Vec<Account> {\n    let _ = slug\n    [Account { code: \"1000\" }, Account { code: \"4000\" }]\n}\n\nfn main() {\n    println(\"${list_accounts(\"demo\").len()}\")\n}\n",
    );
}

#[test]
fn list_literal_in_return_statement_and_if_arms_of_vec_function() {
    let g = typechecks(
        "fn b(f: bool) -> Vec<int> {\n    if f { return [3, 4] }\n    [1, 2]\n}\nfn c(f: bool) -> Vec<int> {\n    if f { [3, 4] } else { [5] }\n}\nfn main() {\n    println(\"${b(true).len()} ${c(true).len()}\")\n}\n",
    );
    assert!(!g.contains("-> Vec<i64> {\n    if f {\n        return [3"), "{g}");
}

#[test]
fn list_literal_inside_closure_in_vec_function_is_unaffected() {
    // the closure's own tail is not the function's return position
    let g = typechecks(
        "fn d() -> Vec<int> {\n    let f = |x: int| [x, x]\n    let a = f(1)\n    println(\"${a.len()}\")\n    [7, 8]\n}\nfn main() {\n    println(\"${d().len()}\")\n}\n",
    );
    assert!(g.contains("|x: i64| [x, x]") && g.contains("vec![7_i64, 8_i64]"), "{g}");
}

// ---- owned text bindings of borrowed pieces ----

#[test]
fn split_piece_assigned_to_owned_text_local() {
    typechecks(
        "fn main() {\n    let text = \"alpha beta gamma\"\n    let mut cur = \"\"\n    for w in text.split_whitespace() {\n        if cur.len() == 0 {\n            cur = w\n        } else {\n            cur = \"${cur} ${w}\"\n        }\n    }\n    println(\"${cur}\")\n}\n",
    );
}

#[test]
fn range_slice_bound_to_declared_string() {
    typechecks("fn main() {\n    let s = \"2024-01-15\"\n    let year: string = s[0..4]\n    println(\"${year}\")\n}\n");
}

#[test]
fn map_keys_and_values_collect_into_declared_vec() {
    typechecks(
        "use std::collections::HashMap\n\nfn main() {\n    let mut m: HashMap<string, int> = HashMap::new()\n    m.insert(\"b\", 2)\n    m.insert(\"a\", 1)\n    let mut names: Vec<string> = m.keys().collect()\n    names.sort()\n    let mut sizes: Vec<int> = m.values().collect()\n    sizes.sort()\n    println(\"${names[0]} ${sizes[0]}\")\n}\n",
    );
}

// ---- map lookups ----

#[test]
fn map_get_unwrap_or_float_then_insert() {
    // The binding of `unwrap_or` over `Option<&float>` is an owned value: no dereference. (The
    // `as f32` narrowing the same program still gets is the numeric solver's, tracked in the drift ledger.)
    let (g, _, _) = generate_and_typecheck(
        "use std::collections::HashMap\n\nfn main() {\n    let mut totals: HashMap<string, float> = HashMap::new()\n    let key = \"a\"\n    let cur = totals.get(key).unwrap_or(0.0)\n    totals.insert(key, cur + 1.5)\n    println(\"${totals.len()}\")\n}\n",
    );
    assert!(!g.contains("*cur"), "{g}");
}

#[test]
fn map_get_unwrap_or_int_then_arithmetic() {
    let g = typechecks(
        "use std::collections::HashMap\n\nfn main() {\n    let mut m: HashMap<string, int> = HashMap::new()\n    let key = \"a\"\n    let cur = m.get(key).unwrap_or(0)\n    m.insert(key, cur + 1)\n    println(\"${m.len()}\")\n}\n",
    );
    assert!(!g.contains("*cur"), "{g}");
}

#[test]
fn map_get_in_sort_by_closure_passes_the_comparator_parameters_as_keys() {
    // the comparator parameters are already `&String`: the lookups take them as they are
    let g = typechecks(
        "use std::collections::HashMap\n\nfn main() {\n    let mut totals: HashMap<string, int> = HashMap::new()\n    totals.insert(\"a\", 3)\n    totals.insert(\"b\", 5)\n    let mut names: Vec<string> = [\"a\", \"b\"]\n    names.sort_by(|x, y| {\n        let tx = totals.get(x).unwrap_or(0)\n        let ty = totals.get(y).unwrap_or(0)\n        ty.cmp(tx)\n    })\n    println(\"${names[0]}\")\n}\n",
    );
    assert!(g.contains("totals.get(x)") && !g.contains("*tx"), "{g}");
}

#[test]
fn map_get_unwrap_or_in_retain_predicate() {
    typechecks(
        "use std::collections::HashMap\n\nfn main() {\n    let mut totals: HashMap<string, int> = HashMap::new()\n    totals.insert(\"a\", 3)\n    let mut kept: Vec<string> = [\"a\", \"b\"]\n    kept.retain(|x| totals.get(x).unwrap_or(0) > 1)\n    println(\"${kept.len()}\")\n}\n",
    );
}

// ---- regression guards for shapes reported from larger programs (they type-check today) ----

#[test]
fn string_const_in_vec_literal_and_field_assignment() {
    typechecks(
        "const KIND: string = \"mcp\"\nconst STATUS: string = \"posted\"\n\npub struct Pay {\n    pub status: string,\n    pub note: string,\n}\n\nfn take(rows: Vec<string>) -> int {\n    rows.len()\n}\n\nfn build(name: string, hash: string) -> int {\n    take(vec![name, KIND, hash])\n}\n\nfn post(note: string) -> Pay {\n    let mut p = Pay { status: \"a\", note: \"b\" }\n    p.status = STATUS\n    p.note = note\n    p\n}\n\nfn main() {\n    println(\"${build(\"n\", \"h\")} ${post(\"z\").status}\")\n}\n",
    );
}

#[test]
fn if_expression_operand_in_tail_assignment_keeps_its_value() {
    let g = typechecks(
        "fn parse(s: string) -> Result<int, string> {\n    if s.is_empty() { return Err(\"empty\") }\n    Ok(5)\n}\n\nfn credit_total(text: string) -> Result<int, string> {\n    let mut cents = 0\n    if !text.is_empty() {\n        let c = match parse(text) {\n            Ok(c) => c,\n            Err(e) => return Err(\"bad: \" + e),\n        }\n        cents = cents + (if c < 0 { 0 - c } else { c })\n    }\n    Ok(cents)\n}\n\nfn main() {\n    println(\"${credit_total(\"x\").unwrap()}\")\n}\n",
    );
    assert!(!g.contains("let _ = 0_i64 - c"), "{g}");
}

// ---- runtime-returned pieces and string matches ----

#[test]
fn strings_chars_collect_goes_through_into_iter() {
    // the generated crate needs the runtime to type-check, so assert on the emitted call chain
    let (g, _, _) = generate_and_typecheck(
        "use std::strings\n\nfn count_a(text: string) -> int {\n    let cs: Vec<char> = strings.chars(text).collect()\n    let mut n = 0\n    for c in cs {\n        if c == 'a' {\n            n = n + 1\n        }\n    }\n    n\n}\n\nfn main() {\n    println(\"${count_a(\"banana\")}\")\n}\n",
    );
    assert!(g.contains("strings::chars(&text).into_iter().collect::<Vec<char>>()"), "{g}");
}

#[test]
fn match_on_cloned_text_as_str_keeps_the_str_scrutinee() {
    let g = typechecks(
        "pub fn parse_len(text: string) -> int {\n    text.len()\n}\n\npub fn total() -> int {\n    let mut n = 0\n    for text in [\"a\", \"bb\", \"ccc\"] {\n        n = n + parse_len(text.clone())\n    }\n    n\n}\n\npub fn label(text: string) -> string {\n    match text.clone().as_str() {\n        \"a\" => \"first\",\n        _ => \"other\",\n    }\n}\n\nfn main() {\n    println(\"${total()} ${label(\"a\")}\")\n}\n",
    );
    assert!(g.contains(".as_str()"), "{g}");
}

// ---- spec-oracle findings ----

#[test]
fn split_once_with_a_text_literal_borrows_the_pattern() {
    let g = typechecks(
        "fn main() {\n    let kv = \"key=value\"\n    match kv.split_once(\"=\") {\n        Some((k, v)) => println(\"${k} -> ${v}\"),\n        None => println(\"no separator\"),\n    }\n    println(\"${\"x=1\".split_once(\"=\").is_some()} ${\"x\".split_once(\"=\").is_none()}\")\n}\n",
    );
    assert!(!g.contains("split_once(String::from"), "{g}");
}

#[test]
fn interpolated_if_and_match_produce_their_branch_values() {
    let g = typechecks(
        "fn t(tag: string, v: int) -> int {\n    println(\"eval ${tag}\")\n    v\n}\n\nfn main() {\n    println(\"${if t(\"c\", 1) == 1 { t(\"then\", 10) } else { t(\"else\", 20) }}\")\n    println(\"${match t(\"m\", 2) { 1 => t(\"one\", 100), _ => t(\"other\", 200) }}\")\n}\n",
    );
    assert!(!g.contains("t(\"then\", 10_i64);"), "{g}");
}

#[test]
fn copied_on_an_option_of_string_slices_is_the_identity() {
    let g = typechecks(
        "fn first_word(s: string) -> bool {\n    let w = s.split(\" \").next()\n    let x = w.copied()\n    match x {\n        Some(t) => t == \"ab\",\n        None => false,\n    }\n}\n\nfn second(s: string) -> bool {\n    match s.split(\" \").last().cloned() {\n        Some(t) => t == \"cde\",\n        None => false,\n    }\n}\n\nfn main() {\n    println(\"${first_word(\"ab cd\")} ${second(\"ab cde\")}\")\n}\n",
    );
    assert!(!g.contains(".copied()") && !g.contains(".cloned()"), "{g}");
}

#[test]
fn method_call_inside_interpolation_makes_the_binding_mutable() {
    let g = typechecks(
        "struct Counter { n: int }\nimpl Counter {\n    fn bump(self) -> int {\n        self.n = self.n + 1\n        self.n\n    }\n    fn get(self) -> int {\n        self.n\n    }\n}\nstruct Wrap { c: Counter }\nimpl Wrap {\n    fn go(self) -> int {\n        self.c.bump()\n    }\n}\nfn main() {\n    let w = Wrap { c: Counter { n: 0 } }\n    println(\"${w.go()}\")\n    let c = Counter { n: 0 }\n    println(\"${c.get()}\")\n}\n",
    );
    assert!(g.contains("let mut w") && g.contains("let c ="), "{g}");
}

// ---- game-engine findings (neutral ports) ----

#[test]
fn if_let_over_split_last_does_not_copy_a_string_slice() {
    let g = typechecks(
        "pub fn extension(path: string) -> string {\n    if let Some(ext) = path.split('.').last() {\n        return ext.to_string()\n    }\n    \"\".to_string()\n}\nfn main() {\n    println(extension(\"a.txt\"))\n}\n",
    );
    assert!(!g.contains(".copied()"), "{g}");
}

#[test]
fn match_over_map_get_with_default_binds_a_value_not_a_reference() {
    let g = typechecks(
        "use std::collections::HashMap\n\npub fn cost(g: HashMap<(i32, i32), f32>, x: i32, y: i32) -> f32 {\n    let cur = match g.get((x, y)) {\n        Some(v) => v,\n        None => 999999.0,\n    }\n    cur + 1.0\n}\nfn main() {\n    println(\"${cost(HashMap::new(), 1, 2)}\")\n}\n",
    );
    assert!(!g.contains("*cur"), "{g}");
}

// ---- optional chaining, list literals inside Ok/Some ----

#[test]
fn optional_chaining_on_a_field_and_a_method() {
    let g = typechecks(
        "struct Item {\n    qty: int,\n}\n\nfn main() {\n    let a: Item? = Some(Item { qty: 5 })\n    let b: Item? = None\n    let qa = a?.qty\n    let qb = b?.qty\n    println(\"${qa ?? 0} ${qa.is_some()}\")\n    println(\"${qb ?? 0} ${qb.is_none()}\")\n}\n",
    );
    assert!(g.contains(".as_ref().map("), "{g}");
}

#[test]
fn optional_chaining_flattens_through_option_members_and_calls_methods() {
    typechecks(
        "struct Profile {\n    nick: string?,\n}\n\nstruct User {\n    name: string,\n    profile: Profile?,\n}\n\nimpl User {\n    fn name_len(self) -> int {\n        self.name.len() as int\n    }\n}\n\nfn main() {\n    let full: User? = Some(User { name: \"abc\", profile: Some(Profile { nick: Some(\"ann\") }) })\n    let no_profile: User? = Some(User { name: \"cy\", profile: None })\n    println(full?.profile?.nick ?? \"anon\")\n    println(no_profile?.profile?.nick ?? \"anon\")\n    println(\"${full?.name_len() ?? 0}\")\n}\n",
    );
}

#[test]
fn list_literal_inside_ok_is_a_vec() {
    typechecks(
        "fn wrap() -> Result<Vec<int>, string> {\n    Ok([1, 2])\n}\n\nfn main() {\n    let r: Result<Vec<int>, string> = Ok([1])\n    println(\"${r.is_ok()} ${wrap().is_ok()}\")\n}\n",
    );
}

#[test]
fn match_on_an_integer_without_a_wildcard_panics_at_run_time_instead_of_failing_to_build() {
    let g = typechecks(
        "fn f(n: int) -> string {\n    match n {\n        1 => \"one\",\n        2 => \"two\",\n    }\n}\nfn main() {\n    println(\"${f(1)}\")\n}\n",
    );
    assert!(g.contains("panic!(\"non-exhaustive match\")"), "{g}");
}

// ---- self mutability through loops and conditions ----

#[test]
fn self_mutability_reaches_a_caller_through_return_in_loop_and_while_condition() {
    let g = typechecks(
        "struct Srv {\n    hits: Vec<int>,\n}\n\nimpl Srv {\n    fn record(self, v: int) -> int {\n        if v > 0 {\n            self.hits.push(v)\n            return 1\n        }\n        0\n    }\n\n    fn step(self) -> bool {\n        loop {\n            return self.record(1) > 5\n        }\n    }\n\n    fn serve_client(self) -> int {\n        let mut n = 0\n        while self.step() {\n            n = n + 1\n        }\n        n\n    }\n}\n\nfn main() {\n    let s = Srv { hits: [] }\n    println(\"${s.serve_client()}\")\n}\n",
    );
    assert!(g.contains("fn serve_client(&mut self)") && g.contains("fn step(&mut self)"), "{g}");
}

// ---- book findings ----

#[test]
fn filter_collect_over_a_collection_copies_the_items() {
    typechecks("fn main() {\n    let v = [5, 3, 8, 4]\n    let evens: Vec<int> = v.filter(|x| *x % 2 == 0).collect()\n    println(\"${evens.len()}\")\n}\n");
}

#[test]
fn entry_with_a_borrowed_text_key_takes_an_owned_key() {
    typechecks(
        "use std::collections::HashMap\n\nfn main() {\n    let mut counts: HashMap<string, int> = HashMap::new()\n    for w in [\"a\", \"b\", \"a\"] {\n        counts.entry(w).and_modify(|c| { *c = *c + 1 }).or_insert(1)\n    }\n    println(\"${counts.len()}\")\n}\n",
    );
}

#[test]
fn map_err_to_a_literal_builds_an_owned_string_error() {
    typechecks("fn parse(text: string) -> Result<int, string> {\n    text.parse::<int>().map_err(|_| \"bad number\")\n}\nfn main() {\n    println(\"${parse(\"4\").is_ok()}\")\n}\n");
}

#[test]
fn trait_default_method_returns_an_owned_string_literal() {
    typechecks(
        "trait Shape {\n    fn area(self) -> float\n    fn name(self) -> string { \"shape\" }\n}\nstruct Sq { s: float }\nimpl Shape for Sq {\n    fn area(self) -> float { self.s * self.s }\n}\nfn main() {\n    let a = Sq { s: 2.0 }\n    println(\"${a.name()} ${a.area()}\")\n}\n",
    );
}

// ---- compiler-consumed attributes and async fn ----

#[test]
fn async_fn_keyword_is_emitted_so_await_compiles() {
    let g = typechecks("async fn fetch(n: int) -> int {\n    n * 2\n}\n\nasync fn total() -> int {\n    let a = fetch(4).await\n    let b = fetch(5).await\n    a + b\n}\n\nfn main() {\n    println(\"parsed\")\n}\n");
    assert!(g.contains("async fn fetch") && g.contains("async fn total"), "{g}");
}

#[test]
fn pure_and_uses_are_consumed_by_the_compiler_not_emitted() {
    let g = typechecks("@pure\nfn add(a: int, b: int) -> int {\n    a + b\n}\n\nfn main() {\n    println(\"${add(1, 2)}\")\n}\n");
    assert!(!g.contains("#[pure]") && !g.contains("#[uses"), "{g}");
}

#[test]
fn value_returned_by_a_spawned_closure_keeps_its_result_type_through_join() {
    let (g, _, _) = generate_and_typecheck(
        "use std::thread\n\nfn work(n: int) -> Result<int, string> {\n    Ok(n + 1)\n}\n\nfn main() {\n    let h = thread::spawn(move || work(2))\n    match h.join() {\n        Ok(served) => println(\"${served.unwrap_or_else(|| -1)}\"),\n        Err(_) => println(\"panicked\"),\n    }\n}\n",
    );
    assert!(g.contains("unwrap_or_else(|_|"), "{g}");
}

#[test]
fn match_over_a_tuple_of_text_with_literal_patterns_compiles() {
    typechecks(
        "fn route(method: string, path: string) -> string {\n    match (method, path) {\n        (\"GET\", \"/x\") => \"x\",\n        (\"POST\", \"/y\") => \"y\",\n        _ => \"none\",\n    }\n}\nfn main() {\n    println(\"${route(\"GET\", \"/x\")} ${route(\"PUT\", \"/z\")}\")\n}\n",
    );
}

#[test]
fn string_literal_with_a_format_spec_at_the_start_of_an_interpolation() {
    let g = typechecks("fn main() {\n    println(\"${\"item\":<8}|\")\n}\n");
    assert!(g.contains("{:<8}") && g.contains("\"item\""), "{g}");
}

#[test]
fn const_declared_in_an_inline_module_is_emitted_and_readable() {
    let g = typechecks("mod outer {\n    pub const LIMIT: int = 10\n\n    fn secret() -> int {\n        LIMIT * 2\n    }\n\n    pub fn reveal() -> int {\n        secret() + 1\n    }\n}\n\nfn main() {\n    println(\"${outer::reveal()} ${outer::LIMIT}\")\n}\n");
    assert!(g.contains("const LIMIT"), "{g}");
}

#[test]
fn map_get_with_a_text_default_over_owned_text_values_compiles() {
    typechecks("use std::collections::BTreeMap\n\nfn main() {\n    let mut groups: BTreeMap<string, string> = BTreeMap::new()\n    let first = \"a\".to_string()\n    let so_far = groups.get(first).unwrap_or(\"\")\n    groups.insert(first, so_far + \"x\")\n    println(\"${groups.len()}\")\n}\n");
}

/// A library module that never names a runtime type but gets a value of it from another module: the type its
/// inferred annotation spells (`Vec<Row>`) must be imported there.
#[test]
fn library_module_annotating_a_runtime_type_it_never_names_imports_it() {
    let dir = TempDir::new().unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(dir.path().join("wj.toml"), "[package]\nname = \"rowrepro\"\nversion = \"0.1.0\"\n\n[lib]\npath = \"src/mod.wj\"\n").unwrap();
    std::fs::write(src.join("mod.wj"), "pub mod helper\npub mod session\npub mod user\n").unwrap();
    std::fs::write(src.join("helper.wj"), "use std::db::Row\n\npub fn col_string(row: Row, col: string) -> (Row, string) {\n    let value = match row.get_string(col) {\n        Ok(v) => v,\n        Err(_) => \"\",\n    }\n    (row, value)\n}\n").unwrap();
    std::fs::write(src.join("session.wj"), "use std::db\n\npub fn connect(url: string) -> Result<Connection, string> {\n    db.connect(url)\n}\n").unwrap();
    std::fs::write(src.join("user.wj"), "use crate::session::connect\nuse crate::helper::col_string\n\npub fn first_id(url: string) -> Result<string, string> {\n    let conn = connect(url)?\n    let rows = conn.query(\"SELECT 1 AS id\", [])?\n    if rows.is_empty() {\n        return Err(\"no rows\")\n    }\n    let (_row, id) = col_string(rows[0], \"id\")\n    Ok(id)\n}\n").unwrap();
    let out = dir.path().join("out");
    let b = Command::new(WJ).args(["build", "--no-cargo", "--no-lint", "--module-file", "-o"]).arg(&out).arg(src.join("mod.wj")).output().unwrap();
    assert!(b.status.success(), "{}", String::from_utf8_lossy(&b.stderr));
    let user = std::fs::read_to_string(out.join("user.rs")).unwrap();
    if user.contains("<Row>") {
        assert!(user.contains("use windjammer_runtime::db::Row;"), "an annotation names Row without importing it:\n{user}");
    }
}
