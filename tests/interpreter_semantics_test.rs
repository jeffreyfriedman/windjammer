#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "interpreter_tests",
))]

//! Interpreter semantics: every test pins Rust-equivalent behaviour (integer widths and
//! overflow panics, formatting, ownership-irrelevant value semantics, error spans).
//! Expected values are hand-derived from Rust semantics, not from the interpreter's output.
//! Differential tests against compiled Rust live in `interpreter_differential_test.rs`.

use windjammer::interpreter::Interpreter;

fn run(source: &str) -> Result<String, String> {
    let mut lexer = windjammer::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser =
        windjammer::parser::Parser::new_with_source(tokens, "test.wj".to_string(), source.to_string());
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    let mut interp = Interpreter::new_capturing();
    interp.run(&program)?;
    Ok(interp.get_output())
}

fn expect(source: &str, want: &str) {
    match run(source) {
        Ok(out) => assert_eq!(out, want, "program:\n{}", source),
        Err(e) => panic!("interpreter failed: {}\nprogram:\n{}", e, source),
    }
}

fn expect_err(source: &str, needle: &str) {
    match run(source) {
        Ok(out) => panic!("expected error containing {:?}, got output {:?}\nprogram:\n{}", needle, out, source),
        Err(e) => assert!(e.contains(needle), "error {:?} does not contain {:?}", e, needle),
    }
}

// ---------------------------------------------------------------- integers

#[test]
fn i32_add_overflow_panics() {
    expect_err(
        "fn main() {\n    let x: i32 = 2147483647\n    let y = x + 1\n    println(\"{}\", y)\n}\n",
        "attempt to add with overflow",
    );
}

#[test]
fn u8_sub_overflow_panics() {
    expect_err(
        "fn main() {\n    let a: u8 = 3\n    let b: u8 = 5\n    println(\"{}\", a - b)\n}\n",
        "attempt to subtract with overflow",
    );
}

#[test]
fn i64_mul_overflow_panics() {
    expect_err(
        "fn main() {\n    let a: i64 = 9223372036854775807\n    println(\"{}\", a * 2)\n}\n",
        "attempt to multiply with overflow",
    );
}

#[test]
fn division_by_zero_panics() {
    expect_err("fn main() {\n    let z = 0\n    println(\"{}\", 10 / z)\n}\n", "attempt to divide by zero");
}

#[test]
fn wrapping_checked_saturating() {
    expect(
        "fn main() {\n    let a: i32 = 2147483647\n    println(\"{}\", a.wrapping_add(1))\n    println(\"{:?}\", a.checked_add(1))\n    println(\"{:?}\", 5i32.checked_add(1))\n    println(\"{}\", a.saturating_add(10))\n    let b: u8 = 200\n    println(\"{}\", b.wrapping_add(100))\n}\n",
        "-2147483648\nNone\nSome(6)\n2147483647\n44\n",
    );
}

#[test]
fn casts_wrap_and_saturate() {
    expect(
        "fn main() {\n    let n: i32 = 300\n    println(\"{}\", n as u8)\n    let m: i32 = -1\n    println(\"{}\", m as u32)\n    let f: f64 = 3.99\n    println(\"{}\", f as i32)\n    let big: f64 = 1e20\n    println(\"{}\", big as i32)\n    let c: u8 = 255\n    println(\"{}\", c as i8)\n}\n",
        "44\n4294967295\n3\n2147483647\n-1\n",
    );
}

#[test]
fn shift_overflow_panics_and_shifts_work() {
    expect(
        "fn main() {\n    let x: u32 = 1\n    println(\"{}\", x << 31)\n    println(\"{}\", 0xF0u8 >> 4)\n}\n",
        "2147483648\n15\n",
    );
    expect_err(
        "fn main() {\n    let x: u32 = 1\n    let s: u32 = 32\n    println(\"{}\", x << s)\n}\n",
        "attempt to shift left with overflow",
    );
}

#[test]
fn integer_constants_and_methods() {
    expect(
        "fn main() {\n    println(\"{}\", i32::MAX)\n    println(\"{}\", u64::MAX)\n    println(\"{}\", i8::MIN)\n    println(\"{}\", 17.rem_euclid(5))\n    println(\"{}\", (-17).rem_euclid(5))\n    println(\"{}\", 2.pow(10))\n}\n",
        "2147483647\n18446744073709551615\n-128\n2\n3\n1024\n",
    );
}

// ---------------------------------------------------------------- formatting

#[test]
fn float_display_matches_rust() {
    expect(
        "fn main() {\n    println(\"{}\", 1.0)\n    println(\"{}\", 0.1 + 0.2)\n    println(\"{:?}\", 1.0)\n    println(\"{:.2}\", 3.14159)\n    println(\"{}\", 1.5e10)\n    println(\"{}\", 100.0 / 3.0)\n}\n",
        "1\n0.30000000000000004\n1.0\n3.14\n15000000000\n33.333333333333336\n",
    );
}

#[test]
fn format_specs() {
    expect(
        "fn main() {\n    println(\"[{:>5}]\", 42)\n    println(\"[{:<5}]\", 42)\n    println(\"[{:^5}]\", 42)\n    println(\"[{:05}]\", 42)\n    println(\"[{:+}]\", 42)\n    println(\"[{:x}]\", 255)\n    println(\"[{:#b}]\", 5)\n    println(\"[{:08.3}]\", 3.14159)\n    println(\"[{:*^7}]\", \"ab\")\n}\n",
        "[   42]\n[42   ]\n[ 42  ]\n[00042]\n[+42]\n[ff]\n[0b101]\n[0003.142]\n[**ab***]\n",
    );
}

#[test]
fn string_interpolation_and_escaped_braces() {
    expect(
        "fn main() {\n    let name = \"wj\"\n    let n = 3\n    println(\"hello ${name} x${n + 1}\")\n    println(\"{{literal}} {}\", n)\n}\n",
        "hello wj x4\n{literal} 3\n",
    );
}

#[test]
fn debug_formatting_of_collections_and_structs() {
    expect(
        "struct P {\n    x: int,\n    y: int,\n}\nfn main() {\n    let v = vec![1, 2, 3]\n    println(\"{:?}\", v)\n    let o = Some(\"a\")\n    println(\"{:?}\", o)\n    let p = P { x: 1, y: 2 }\n    println(\"{:?}\", p)\n    println(\"{:?}\", (1, \"b\"))\n}\n",
        "[1, 2, 3]\nSome(\"a\")\nP { x: 1, y: 2 }\n(1, \"b\")\n",
    );
}

// ---------------------------------------------------------------- strings

#[test]
fn string_methods() {
    expect(
        "fn main() {\n    let s = \"  Hello, World  \"\n    let t = s.trim()\n    println(\"{}\", t)\n    println(\"{}\", t.to_uppercase())\n    println(\"{}\", t.replace(\"World\", \"WJ\"))\n    println(\"{}\", t.len())\n    println(\"{}\", t.starts_with(\"Hello\"))\n    let parts: Vec<string> = \"a,b,c\".split(\",\").map(|p| p.to_string()).collect()\n    println(\"{}\", parts.len())\n    println(\"{}\", parts.join(\"-\"))\n    let rev: string = \"abc\".chars().rev().collect()\n    println(\"{}\", rev)\n}\n",
        "Hello, World\nHELLO, WORLD\nHello, WJ\n12\ntrue\n3\na-b-c\ncba\n",
    );
}

#[test]
fn parse_with_turbofish_and_errors() {
    expect(
        "fn main() {\n    let a = \"42\".parse::<i32>().unwrap()\n    println(\"{}\", a + 1)\n    let b = \"x\".parse::<i32>()\n    println(\"{}\", b.is_err())\n    let c = \"300\".parse::<u8>()\n    println(\"{}\", c.is_err())\n    let d = \"2.5\".parse::<f64>().unwrap()\n    println(\"{}\", d * 2.0)\n}\n",
        "43\ntrue\ntrue\n5\n",
    );
}

// ---------------------------------------------------------------- functions & closures

#[test]
fn recursion_and_mutual_recursion() {
    expect(
        "fn fib(n: int) -> int {\n    if n < 2 {\n        return n\n    }\n    fib(n - 1) + fib(n - 2)\n}\nfn is_even(n: int) -> bool {\n    if n == 0 { true } else { is_odd(n - 1) }\n}\nfn is_odd(n: int) -> bool {\n    if n == 0 { false } else { is_even(n - 1) }\n}\nfn main() {\n    println(\"{}\", fib(20))\n    println(\"{}\", is_even(10))\n}\n",
        "6765\ntrue\n",
    );
}

#[test]
fn closures_capture_and_mutate_environment() {
    expect(
        "fn main() {\n    let mut count = 0\n    let mut inc = || {\n        count += 1\n    }\n    inc()\n    inc()\n    inc()\n    println(\"{}\", count)\n    let base = 10\n    let add = |x| x + base\n    println(\"{}\", add(5))\n}\n",
        "3\n15\n",
    );
}

#[test]
fn closures_passed_to_functions() {
    expect(
        "fn apply(f: fn(int) -> int, x: int) -> int {\n    f(x)\n}\nfn main() {\n    let k = 3\n    println(\"{}\", apply(|v| v * k, 7))\n    println(\"{}\", apply(|v| v - 1, 7))\n}\n",
        "21\n6\n",
    );
}

// ---------------------------------------------------------------- collections & iterators

#[test]
fn iterator_chains() {
    expect(
        "fn main() {\n    let v = vec![1, 2, 3, 4, 5]\n    let s: int = v.iter().map(|x| x * 2).filter(|x| *x > 4).sum()\n    println(\"{}\", s)\n    let evens: Vec<int> = v.iter().filter(|x| *x % 2 == 0).cloned().collect()\n    println(\"{:?}\", evens)\n    let total = v.iter().fold(0, |acc, x| acc + x)\n    println(\"{}\", total)\n    for (i, x) in v.iter().enumerate() {\n        if i == 1 {\n            println(\"{} {}\", i, x)\n        }\n    }\n    println(\"{}\", v.iter().any(|x| *x == 3))\n    println(\"{}\", v.iter().all(|x| *x < 5))\n    println(\"{:?}\", v.iter().max())\n    println(\"{:?}\", v.iter().position(|x| *x == 4))\n}\n",
        "24\n[2, 4]\n15\n1 2\ntrue\nfalse\nSome(5)\nSome(3)\n",
    );
}

#[test]
fn vec_operations() {
    expect(
        "fn main() {\n    let mut v = vec![5, 3, 8, 1, 9, 3]\n    v.sort()\n    println(\"{:?}\", v)\n    v.dedup()\n    println(\"{:?}\", v)\n    v.retain(|x| *x > 2)\n    println(\"{:?}\", v)\n    v.push(7)\n    let last = v.pop()\n    println(\"{:?} {}\", last, v.len())\n    v.reverse()\n    println(\"{:?}\", v)\n    v.insert(1, 100)\n    v.remove(0)\n    println(\"{:?}\", v)\n}\n",
        "[1, 3, 3, 5, 8, 9]\n[1, 3, 5, 8, 9]\n[3, 5, 8, 9]\nSome(7) 4\n[9, 8, 5, 3]\n[100, 8, 5, 3]\n",
    );
}

#[test]
fn hashmap_entry_counts() {
    expect(
        "use std::collections::HashMap\nfn main() {\n    let words = vec![\"a\", \"b\", \"a\", \"c\", \"a\", \"b\"]\n    let mut counts: HashMap<string, int> = HashMap::new()\n    for w in words {\n        *counts.entry(w.to_string()).or_insert(0) += 1\n    }\n    let mut keys: Vec<string> = counts.keys().cloned().collect()\n    keys.sort()\n    for k in keys {\n        println(\"{} {}\", k, counts[&k])\n    }\n    println(\"{}\", counts.contains_key(\"z\"))\n}\n",
        "a 3\nb 2\nc 1\nfalse\n",
    );
}

#[test]
fn vec_index_out_of_bounds_reports_span() {
    match run("fn main() {\n    let v = vec![1, 2, 3]\n    let i = 7\n    println(\"{}\", v[i])\n}\n") {
        Ok(o) => panic!("expected panic, got {:?}", o),
        Err(e) => {
            assert!(e.contains("index out of bounds: the len is 3 but the index is 7"), "{}", e);
            assert!(e.contains("test.wj:4"), "error must carry the source line: {}", e);
        }
    }
}

// ---------------------------------------------------------------- Option / Result / ?

#[test]
fn option_and_result_combinators() {
    expect(
        "fn half(n: int) -> Option<int> {\n    if n % 2 == 0 { Some(n / 2) } else { None }\n}\nfn main() {\n    println(\"{:?}\", half(8).map(|x| x + 1))\n    println(\"{:?}\", half(7).unwrap_or(-1))\n    println(\"{:?}\", half(8).and_then(half))\n    let r: Result<int, string> = Ok(3)\n    println(\"{:?}\", r.map(|x| x * 2))\n    let e: Result<int, string> = Err(\"bad\".to_string())\n    println(\"{:?}\", e.map_err(|m| m.len()))\n    println(\"{}\", e.is_err())\n}\n",
        "Some(5)\n-1\nSome(2)\nOk(6)\nErr(3)\ntrue\n",
    );
}

#[test]
fn question_mark_propagates_errors() {
    expect(
        "fn parse(s: string) -> Result<int, string> {\n    let r = s.parse::<int>()\n    match r {\n        Ok(n) => Ok(n),\n        Err(_) => Err(\"bad number: \" + s),\n    }\n}\nfn sum(a: string, b: string) -> Result<int, string> {\n    let x = parse(a)?\n    let y = parse(b)?\n    Ok(x + y)\n}\nfn main() {\n    println(\"{:?}\", sum(\"1\", \"2\"))\n    println(\"{:?}\", sum(\"1\", \"zz\"))\n}\n",
        "Ok(3)\nErr(\"bad number: zz\")\n",
    );
}

#[test]
fn unwrap_on_none_panics_with_rust_message() {
    expect_err(
        "fn main() {\n    let o: Option<int> = None\n    println(\"{}\", o.unwrap())\n}\n",
        "called `Option::unwrap()` on a `None` value",
    );
}

// ---------------------------------------------------------------- user types

#[test]
fn struct_mutation_through_inferred_mut_parameter() {
    expect(
        "struct Counter {\n    value: int,\n}\nfn bump(c: Counter) {\n    c.value += 10\n}\nimpl Counter {\n    fn inc(self) {\n        self.value += 1\n    }\n    fn get(self) -> int {\n        self.value\n    }\n}\nfn main() {\n    let mut c = Counter { value: 0 }\n    c.inc()\n    c.inc()\n    bump(c)\n    println(\"{}\", c.get())\n}\n",
        "12\n",
    );
}

#[test]
fn copy_values_are_independent() {
    expect(
        "struct P {\n    x: int,\n}\nfn main() {\n    let a = P { x: 1 }\n    let mut b = a.clone()\n    b.x = 2\n    println(\"{} {}\", a.x, b.x)\n    let t = (1, 2)\n    let mut u = t\n    u.0 = 9\n    println(\"{} {}\", t.0, u.0)\n}\n",
        "1 2\n1 9\n",
    );
}

#[test]
fn enums_with_data_guards_and_if_let() {
    expect(
        "enum Shape {\n    Circle(int),\n    Rect { w: int, h: int },\n    Unit,\n}\nfn area(s: Shape) -> int {\n    match s {\n        Shape::Circle(r) => 3 * r * r,\n        Shape::Rect { w, h } if w == h => w * w,\n        Shape::Rect { w, h } => w * h,\n        Shape::Unit => 0,\n    }\n}\nfn main() {\n    println(\"{}\", area(Shape::Circle(2)))\n    println(\"{}\", area(Shape::Rect { w: 3, h: 3 }))\n    println(\"{}\", area(Shape::Rect { w: 3, h: 4 }))\n    println(\"{}\", area(Shape::Unit))\n    let o = Some(5)\n    if let Some(v) = o {\n        println(\"got {}\", v)\n    }\n    let mut stack = vec![1, 2, 3]\n    while let Some(top) = stack.pop() {\n        print(\"{} \", top)\n    }\n    println(\"\")\n}\n",
        "12\n9\n12\n0\ngot 5\n3 2 1 \n",
    );
}

#[test]
fn traits_defaults_and_dynamic_dispatch() {
    expect(
        "trait Animal {\n    fn name(self) -> string\n    fn speak(self) -> string {\n        self.name() + \" makes a sound\"\n    }\n}\nstruct Dog {}\nstruct Cat {}\nimpl Animal for Dog {\n    fn name(self) -> string {\n        \"Dog\".to_string()\n    }\n    fn speak(self) -> string {\n        \"Woof\".to_string()\n    }\n}\nimpl Animal for Cat {\n    fn name(self) -> string {\n        \"Cat\".to_string()\n    }\n}\nfn describe<T: Animal>(a: T) -> string {\n    a.speak()\n}\nfn main() {\n    println(\"{}\", describe(Dog {}))\n    println(\"{}\", describe(Cat {}))\n}\n",
        "Woof\nCat makes a sound\n",
    );
}

#[test]
fn generic_struct_and_impl() {
    expect(
        "struct Stack<T> {\n    items: Vec<T>,\n}\nimpl<T> Stack<T> {\n    fn new() -> Stack<T> {\n        Stack { items: Vec::new() }\n    }\n    fn push(self, x: T) {\n        self.items.push(x)\n    }\n    fn pop(self) -> Option<T> {\n        self.items.pop()\n    }\n    fn len(self) -> int {\n        self.items.len() as int\n    }\n}\nfn main() {\n    let mut s: Stack<int> = Stack::new()\n    s.push(1)\n    s.push(2)\n    println(\"{}\", s.len())\n    println(\"{:?}\", s.pop())\n    println(\"{}\", s.len())\n}\n",
        "2\nSome(2)\n1\n",
    );
}

// ---------------------------------------------------------------- diagnostics

#[test]
fn unsupported_features_fail_loudly_not_silently() {
    // A program using something the interpreter cannot do must produce an explicit error.
    match run("fn main() {\n    let x = missing_function(3)\n    println(\"{}\", x)\n}\n") {
        Ok(o) => panic!("silently produced {:?}", o),
        Err(e) => assert!(e.contains("missing_function"), "{}", e),
    }
}

#[test]
fn main_exit_value_and_explicit_panic() {
    expect_err("fn main() {\n    panic(\"boom {}\", 7)\n}\n", "boom 7");
}

// ---------------------------------------------------------------- ADR-0014 Phase 1 intrinsics
// Expected texts are hand-derived from rustc's panic/format messages (verified against compiled
// output in interpreter_differential_test.rs), never from the interpreter's own output.

fn main_with(body: &str) -> String {
    format!("fn main() {{\n    let x = 5\n    let f = 2.0\n    let s = \"s\"\n    {}\n}}\n", body)
}

#[test]
fn intrinsic_panic_family_messages_match_rust() {
    for (body, want) in [
        ("panic()", "explicit panic"),
        ("panic(\"boom ${x}\")", "boom 5"),
        ("todo()", "not yet implemented"),
        ("todo(\"later ${x}\")", "not yet implemented: later 5"),
        ("unreachable()", "internal error: entered unreachable code"),
        ("unreachable(\"state ${x}\")", "internal error: entered unreachable code: state 5"),
        ("assert(x == 6)", "assertion failed: x == 6"),
        ("assert(x == 6, \"x was ${x}\")", "x was 5"),
        ("assert_eq(x, 6)", "assertion `left == right` failed\n  left: 5\n right: 6"),
        ("assert_eq(x, 6, \"ctx ${x}\")", "assertion `left == right` failed: ctx 5\n  left: 5\n right: 6"),
        ("assert_ne(x, 5)", "assertion `left != right` failed\n  left: 5\n right: 5"),
        ("assert_ne(x, 5, \"same\")", "assertion `left != right` failed: same\n  left: 5\n right: 5"),
        ("assert_eq(\"a\", \"b\")", "left: \"a\"\n right: \"b\""),
    ] {
        match run(&main_with(body)) {
            Ok(o) => panic!("{} must panic, printed {:?}", body, o),
            Err(e) => assert!(e.contains(want), "{}: error {:?} lacks {:?}", body, e, want),
        }
    }
}

#[test]
fn legacy_macro_spellings_behave_like_the_canonical_calls() {
    for (legacy, canon) in [
        ("println!(\"v={} {:?}\", x, s)", "println(\"v=${x} ${s:?}\")"),
        ("print!(\"a{}\", x)", "print(\"a${x}\")"),
        ("println!(\"{{}} lit\")", "println(\"{} lit\")"),
        ("assert!(x == 5, \"m {}\", x)", "assert(x == 5, \"m ${x}\")"),
        ("assert_eq!(x, 5)", "assert_eq(x, 5)"),
        ("assert_ne!(x, 6, \"n\")", "assert_ne(x, 6, \"n\")"),
        ("let y = dbg!(x)", "let y = debug(x)"),
        ("let v = vec![1, 2, 3]\n    println(\"${v:?}\")", "let v = [1, 2, 3]\n    println(\"${v:?}\")"),
        ("eprintln!(\"e {}\", x)", "stderr.println(\"e ${x}\")"),
    ] {
        assert_eq!(run(&main_with(legacy)), run(&main_with(canon)), "{} vs {}", legacy, canon);
    }
    // and the failing forms produce the same panic text
    for (legacy, canon) in [
        ("assert!(x == 6)", "assert(x == 6)"),
        ("assert_eq!(x, 6, \"c {}\", x)", "assert_eq(x, 6, \"c ${x}\")"),
        ("todo!(\"w {}\", x)", "todo(\"w ${x}\")"),
        ("unreachable!()", "unreachable()"),
    ] {
        let (a, b) = (run(&main_with(legacy)).unwrap_err(), run(&main_with(canon)).unwrap_err());
        let strip = |s: String| s.split(" (").next().unwrap().to_string();
        assert_eq!(strip(a), strip(b), "{} vs {}", legacy, canon);
    }
}

#[test]
fn interpolation_specs_and_literal_braces() {
    expect(
        &main_with("println(\"${f:.2}|${x:?}|${x:>4}|${x:<4}|${x:04}|${x:x}|${s:?}|{lit}|${x}\")"),
        "2.00|5|   5|5   |0005|5|\"s\"|{lit}|5\n",
    );
    // a plain (non-interpolated) literal is printed verbatim, braces included
    expect(&main_with("println(\"{x} {{y}}\")"), "{x} {{y}}\n");
}

#[test]
fn list_literal_is_a_growable_vec() {
    expect(
        "struct B {\n    items: Vec<int>,\n}\nfn main() {\n    let mut v = [1, 2]\n    v.push(3)\n    let mut b = B { items: [9] }\n    b.items.push(8)\n    let e: Vec<int> = []\n    println(\"${v:?} ${b.items:?} ${e.len()}\")\n}\n",
        "[1, 2, 3] [9, 8] 0\n",
    );
}

#[test]
fn closure_literal_in_call_position_and_pipe() {
    expect(&main_with("println(\"${(|v| v + 1)(x)} ${x |> |v| v * 2} ${x |> |v| v + 1 |> |w| w * 10}\")"), "6 10 60\n");
}

#[test]
fn debug_returns_its_argument_and_stays_off_stdout() {
    // run() captures stdout only: `debug` must not leak into it and must return the value.
    expect(&main_with("let y = debug(x + 1)\n    println(\"${y}\")"), "6\n");
}

#[test]
fn user_definitions_shadow_intrinsic_names() {
    expect(
        "fn debug(n: int) -> int {\n    n * 100\n}\nfn main() {\n    println(\"${debug(2)}\")\n}\n",
        "200\n",
    );
}

#[test]
fn unsupported_intrinsic_shapes_are_explicit_errors() {
    for (body, needle) in [
        ("let y = debug(x, x)", "`debug` takes exactly one argument"),
        ("assert()", "without a condition"),
        ("assert_eq(x)", "needs two operands"),
        ("println(a: x)", "needs a callee declared in this file"),
    ] {
        expect_err(&main_with(body), needle);
    }
}

#[test]
fn format_template_with_unused_argument_is_rejected() {
    expect_err(&main_with("println(\"no placeholder\", x)"), "argument never used");
}
