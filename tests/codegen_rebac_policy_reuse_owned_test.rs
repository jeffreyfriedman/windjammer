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

//! ReBAC-style recursive resolve (mirrors wdb-authz rebac_resolver) + Vec membership:
//! - recursive resolve_check must not pass `&policy` into owned `Policy`
//! - Vec reused after contains-check must borrow or clone (no move-then-push E0382)

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn recursive_owned_policy_call_and_vec_contains_reuse() {
    let source = r#"
pub struct Policy {
    pub tag: string,
}

pub struct Schema {
    pub name: string,
}

impl Policy {
    pub fn check(self, relation: string) -> bool {
        self.tag == relation
    }
}

impl Schema {
    pub fn has(self, relation: string) -> bool {
        self.name == relation
    }
}

fn contains_string(items: Vec<string>, needle: string) -> bool {
    for item in items {
        if item == needle {
            return true
        }
    }
    false
}

pub fn resolve_check(
    policy: Policy,
    schema: Schema,
    relation: string,
    depth: u32,
    max_depth: u32,
) -> bool {
    if depth > max_depth {
        return false
    }
    if policy.check(relation) {
        return true
    }
    if schema.has(relation) {
        if resolve_check(policy, schema, "child", depth + 1, max_depth) {
            return true
        }
    }
    false
}

pub fn list_unique(mut out: Vec<string>, object_id: string) -> Vec<string> {
    if !contains_string(out, object_id) {
        out.push(object_id)
    }
    out
}

fn main() {
    let p = Policy { tag: "t" + "" }
    let s = Schema { name: "n" + "" }
    let _ = resolve_check(p, s, "parent" + "", 0, 2)
    let mut out = Vec::new()
    out = list_unique(out, "a" + "")
    out = list_unique(out, "a" + "")
}
"#;

    let rs = test_utils::compile_single(source);
    let policy_formal_borrowed = rs.contains("resolve_check(policy: &Policy")
        || rs.contains("fn resolve_check(policy: &Policy")
        || rs.contains("pub fn resolve_check(policy: &Policy");
    let policy_call_borrowed = rs.contains("resolve_check(&policy");
    assert!(
        policy_formal_borrowed == policy_call_borrowed,
        "resolve_check policy formal/call-site ownership must agree \
         (borrowed formal={policy_formal_borrowed}, borrowed call={policy_call_borrowed}). Got:\n{rs}"
    );
    if !policy_formal_borrowed {
        assert!(
            !rs.contains("resolve_check(&policy"),
            "owned Policy formal must not get &policy at recursive call. Got:\n{rs}"
        );
    }
    assert!(
        rs.contains("contains_string(&out")
            || rs.contains("contains_string(out.clone()")
            || rs.contains("contains_string(&mut out"),
        "contains_string must not move out before push. Got:\n{rs}"
    );
    test_utils::verify_rust_compiles(&rs).expect("generated Rust must compile");
}
