use super::*;

fn only(f: impl FnOnce(&mut OptimizationConfig)) -> OptimizationConfig {
    let mut c = OptimizationConfig::disabled();
    f(&mut c);
    c
}

fn opt(c: &OptimizationConfig, src: &str) -> String {
    run(c, src).code
}

#[test]
fn disabled_is_identity() {
    let src = "fn f() { let v = vec![1].clone(); }";
    assert_eq!(opt(&OptimizationConfig::disabled(), src), src);
}

#[test]
fn unparsable_input_is_unchanged() {
    let src = "fn f( {";
    assert_eq!(run(&OptimizationConfig::production(), src).code, src);
}

#[test]
fn temp_clone_removes_clone_of_vec_macro() {
    let c = only(|c| c.temp_clone = true);
    assert_eq!(opt(&c, "fn f() { g(vec![1, 2].clone()); }"), "fn f() { g(vec![1, 2]); }");
    assert_eq!(opt(&c, "fn f() { let s = \"a\".to_string().clone(); }"), "fn f() { let s = \"a\".to_string(); }");
}

#[test]
fn temp_clone_keeps_clone_of_variable() {
    let c = only(|c| c.temp_clone = true);
    let src = "fn f(v: Vec<i32>) { let a = v.clone(); g(v); g(a); }";
    assert_eq!(opt(&c, src), src);
}

#[test]
fn copy_clone_removes_on_primitive_binding() {
    let c = only(|c| c.copy_clone = true);
    assert_eq!(
        opt(&c, "fn f(n: usize) { let a = 3_i32; g(n.clone(), a.clone()); }"),
        "fn f(n: usize) { let a = 3_i32; g(n, a); }"
    );
}

#[test]
fn copy_clone_respects_shadowing_and_patterns() {
    let c = only(|c| c.copy_clone = true);
    let shadow = "fn f(n: usize, s: String) { let n = s; g(n.clone()); }";
    assert_eq!(opt(&c, shadow), shadow);
    let pat = "fn f(n: i32, o: Option<String>) { if let Some(n) = o { g(n.clone()); } }";
    assert_eq!(opt(&c, pat), pat);
    let refparam = "fn f(n: &i32) { g(n.clone()); }";
    assert_eq!(opt(&c, refparam), refparam);
}

#[test]
fn same_type_cast_collapses_double_and_identity_casts() {
    let c = only(|c| c.same_type_cast = true);
    assert_eq!(opt(&c, "fn f(n: usize) -> usize { n as usize + 1 }"), "fn f(n: usize) -> usize { n + 1 }");
    assert_eq!(opt(&c, "fn f(x: i64) -> usize { x as i32 as i32 as usize }"), "fn f(x: i64) -> usize { x as i32 as usize }");
    let keep = "fn f(n: i32) -> usize { n as usize }";
    assert_eq!(opt(&c, keep), keep);
}

#[test]
fn with_capacity_before_push_loop() {
    let c = only(|c| c.with_capacity = true);
    let src = "fn f(n: usize) { let mut v: Vec<f64> = Vec::new(); for _i in 0_usize..n * n { v.push(0.0); } }";
    let want = "fn f(n: usize) { let mut v: Vec<f64> = Vec::with_capacity(n * n); for _i in 0_usize..n * n { v.push(0.0); } }";
    assert_eq!(opt(&c, src), want);
}

#[test]
fn with_capacity_rejects_early_exit_nonzero_start_and_impure_bound() {
    let c = only(|c| c.with_capacity = true);
    for src in [
        "fn f(n: usize) { let mut v = Vec::new(); for i in 0_usize..n { v.push(i); if i > 3 { break; } } }",
        "fn f(n: usize) { let mut v = Vec::new(); for i in 1_usize..n { v.push(i); } }",
        "fn f() { let mut v = Vec::new(); for i in 0_usize..g() { v.push(i); } }",
        "fn f(n: usize) { let mut v = Vec::new(); for i in 0_usize..n { if i > 1 { v.push(i); } } }",
        "fn f(n: usize) { let mut v = Vec::new(); for i in 0_usize..n { v.push(i); } println!(\"{}\", v.capacity()); }",
    ] {
        assert_eq!(opt(&c, src), src, "{src}");
    }
}

#[test]
fn string_append_rewrites_self_format() {
    let c = only(|c| c.string_append = true);
    let src = "fn f(ws: Vec<String>) -> String { let mut out = String::new(); for w in ws { out = format!(\"{}{}{}\", out, w, \",\"); } out }";
    let got = opt(&c, src);
    assert!(got.contains("write!(out, \"{}{}\", w, \",\")"), "{got}");
    assert!(!got.contains("format!"));
}

#[test]
fn string_append_rejects_unsafe_shapes() {
    let c = only(|c| c.string_append = true);
    for src in [
        "fn f(mut out: String) { out = format!(\"{}{}\", out, out.len()); }",
        "fn f(mut out: String, w: i32) { out = format!(\"{w}{}\", out, 1); }",
        "fn f(mut out: String) { out = format!(\"{0}{}\", out); }",
        "fn f(out: &mut String) { *out = format!(\"{}x\", out); }",
        "fn f(mut out: Vec<u8>) { out = format!(\"{}x\", out); }",
        "fn f() { let mut o = String::new(); let o = 5; let mut o2 = o; o2 = format!(\"{}x\", o2); }",
    ] {
        assert_eq!(opt(&c, src), src, "{src}");
    }
}

#[test]
fn inline_policy_drops_on_tests_main_and_large_fns() {
    let c = only(|c| {
        c.inline_policy = true;
        c.inline_max_body_bytes = 40;
    });
    let src = "#[inline]\nfn main() {}\n#[test]\n#[inline]\nfn t() {}\n#[inline]\nfn small() -> i32 { 1 }\n#[inline]\nfn big() -> i32 { let a = 1; let b = 2; let c = 3; a + b + c }\n";
    let got = opt(&c, src);
    assert_eq!(got, "\nfn main() {}\n#[test]\n\nfn t() {}\n#[inline]\nfn small() -> i32 { 1 }\n\nfn big() -> i32 { let a = 1; let b = 2; let c = 3; a + b + c }\n");
    assert_eq!(got.lines().count(), src.lines().count(), "line numbers must be preserved");
}

#[test]
fn dead_helpers_removes_private_repr_and_unused_to_bytes() {
    let c = only(|c| c.dead_helpers = true);
    let src = "#[repr(C)]\nstruct P { x: u64 }\nimpl P {\n    pub fn to_bytes(&self) -> Vec<u8> { vec![] }\n}\nfn main() {}\n";
    let got = opt(&c, src);
    assert_eq!(got.lines().count(), src.lines().count(), "line numbers must be preserved");
    assert!(got.contains("struct P { x: u64 }") && !got.contains("repr") && !got.contains("to_bytes"), "{got}");
}

#[test]
fn dead_helpers_keeps_pub_referenced_and_layout_sensitive() {
    let c = only(|c| c.dead_helpers = true);
    let pub_struct = "#[repr(C)]\npub struct P { x: u64 }\nimpl P {\n    pub fn to_bytes(&self) -> Vec<u8> { vec![] }\n}\n";
    assert_eq!(opt(&c, pub_struct), pub_struct);
    let used = "#[repr(C)]\nstruct P { x: u64 }\nimpl P {\n    pub fn to_bytes(&self) -> Vec<u8> { vec![] }\n}\nfn g(p: P) { p.to_bytes(); }\n";
    assert!(opt(&c, used).contains("to_bytes(&self)"));
    let sens = "#[repr(C)]\nstruct P { x: u64 }\nfn g(p: &P) -> usize { std::mem::size_of::<P>() }\n";
    assert_eq!(opt(&c, sens), sens);
}

#[test]
fn production_run_is_idempotent() {
    let src = "fn f(n: usize) -> String { let mut v = Vec::new(); for i in 0_usize..n { v.push(i as usize); } let mut s = String::new(); s = format!(\"{}{}\", s, v.len()); s }";
    let once = run_default(src);
    assert_eq!(run_default(&once), once);
}

#[test]
fn macro_arg_clone_strips_format_family_args_only() {
    let c = only(|c| c.macro_arg_clone = true);
    assert_eq!(
        opt(&c, "fn f(s: &String) { println!(\"{} {}\", s.clone(), 1); let t = format!(\"{}\", s.clone()); }"),
        "fn f(s: &String) { println!(\"{} {}\", s, 1); let t = format!(\"{}\", s); }"
    );
    let keep = "fn f(s: String) { let v = vec![s.clone()]; g(s.clone()); h(s); }";
    assert_eq!(opt(&c, keep), keep);
    let w = "fn f(o: &mut String, s: &String) { write!(o.clone(), \"{}\", s.clone()); }";
    assert_eq!(opt(&c, w), "fn f(o: &mut String, s: &String) { write!(o.clone(), \"{}\", s); }");
}

#[test]
fn with_capacity_skips_sibling_vec_lets_and_handles_literal_i32_bounds() {
    let c = only(|c| c.with_capacity = true);
    let src = "fn f(n: usize) { let mut a = Vec::new(); let mut b = Vec::new(); for _i in 0_usize..n { a.push(1); b.push(2); } }";
    let got = opt(&c, src);
    assert!(got.contains("let mut b = Vec::with_capacity(n)"), "{got}");
    let lit = "fn f() { let mut v: Vec<u8> = Vec::new(); for _i in 0_i32..5000_i32 { v.push(1); } }";
    assert!(opt(&c, lit).contains("Vec::with_capacity(5000_i32 as usize)"));
    let neg_var = "fn f(n: i32) { let mut v: Vec<u8> = Vec::new(); for _i in 0_i32..n { v.push(1); } }";
    assert_eq!(opt(&c, neg_var), neg_var);
}
