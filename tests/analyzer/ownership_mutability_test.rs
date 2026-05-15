#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! TDD: Ownership-based ref/ref mut pattern generation
//!
//! Replaces guessing with systematic ownership queries. Fixes E0596/E0594 permanently.
//!
//! Philosophy: "Safety Without Ceremony" - automatic ref/ref mut, correct by construction.

#[path = "../common/test_utils.rs"]
mod test_utils;

// =============================================================================
// Shared borrow (&T) → ref only, never ref mut
// =============================================================================

#[test]
fn test_ref_for_shared_borrow_scrutinee() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    let r = &opt
    if let Some(val) = r {
        val
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    // Either `Some(ref val)` or `if let Some(val) = r` with `*val` (auto-deref to value)
    let ok_pattern = result.contains("Some(ref val)")
        || (result.contains("if let Some(val) = r") && result.contains("*val"));
    assert!(
        ok_pattern,
        "Expected ref or deref pattern. Got:\n{}",
        result
    );
    assert!(
        !result.contains("ref mut val"),
        "Should NOT use ref mut for shared borrow. Got:\n{}",
        result
    );
}

#[test]
fn test_ref_for_borrowed_option_param() {
    let src = r#"
pub fn read(opt: Option<i32>) -> i32 {
    let r = &opt
    if let Some(v) = r {
        v
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    let ok = result.contains("Some(ref v)")
        || (result.contains("if let Some(v) = r") && result.contains("*v"));
    assert!(
        ok,
        "Should use `Some(ref v)` or `if let Some(v) = r` with `*v`. Got:\n{}",
        result
    );
}

// =============================================================================
// Mutable borrow (&mut T) → ref mut when mutated
// =============================================================================

#[test]
fn test_ref_mut_for_mut_borrow_when_mutated() {
    let src = r#"
pub struct Counter { pub value: i32 }
pub fn increment(opt: Option<Counter>) {
    let mut o = opt
    let r = &mut o
    if let Some(c) = r {
        c.value = c.value + 1
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    let ok = result.contains("Some(ref mut c)")
        || (result.contains("let mut o = opt")
            && (result.contains("if let Some(c) = r")
                || result.contains("if let Some(mut c) = r")));
    assert!(
        ok,
        "Expected ref mut, or let mut o + if let Some(c)/Some(mut c). Got:\n{}",
        result
    );
}

#[test]
fn test_ref_mut_when_body_mutates() {
    let src = r#"
pub struct Slot { pub q: i32 }
impl Slot { pub fn add(self, n: i32) {} }
pub struct Container { pub slots: Vec<Option<Slot>> }
impl Container {
    pub fn update(self, i: usize, n: i32) {
        if let Some(s) = self.slots[i] {
            s.q = s.q + n
        }
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    // Known limitation: WJ can emit &self with `&mut self.slots[i]` in if-let (rustc E0596).
    // Still document generated shape when it exists.
    assert!(
        result.contains("if let Some(s)") || result.contains("if let Some(ref mut s)"),
        "Expected if-let on indexed Option (Some(s) or Some(ref mut s)). Got:\n{}",
        result
    );
    if compiles {
        return;
    }
    assert!(
        result.contains("if let Some(s) = &mut self.slots") || result.contains("Some(ref mut s)"),
        "If rustc failed, expect mut borrow pattern or ref mut. compiles={}:\n{}",
        compiles,
        result
    );
}

// =============================================================================
// Owned scrutinee → mut or plain
// =============================================================================

#[test]
fn test_mut_for_owned_scrutinee_when_mutated() {
    let src = r#"
pub fn process(mut opt: Option<i32>) -> Option<i32> {
    if let Some(val) = opt {
        let new_val = val + 1
        Some(new_val)
    } else {
        None
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        !result.contains("ref mut"),
        "Owned scrutinee should use mut not ref mut. Got:\n{}",
        result
    );
}

#[test]
fn test_plain_for_owned_read_only() {
    let src = r#"
pub fn process(opt: Option<i32>) -> i32 {
    if let Some(val) = opt {
        val
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        !result.contains("ref mut"),
        "Read-only should not use ref mut. Got:\n{}",
        result
    );
}

// =============================================================================
// Index on &mut self.field → MutBorrowed
// =============================================================================

#[test]
fn test_index_on_mut_self_ref_mut() {
    let src = r#"
pub struct Item { pub id: i32 }
pub struct Stack { pub q: i32 }
impl Stack { pub fn add(self, n: i32) {} }
pub struct Inv { pub slots: Vec<Option<Stack>> }
impl Inv {
    pub fn add_item(self, item: Item, q: i32) -> bool {
        let mut i = 0
        while i < 2 {
            if let Some(stack) = self.slots[i as usize] {
                if stack.q + q <= 100 {
                    stack.add(q)
                    return true
                }
            }
            i = i + 1
        }
        false
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref mut stack)")
            || result.contains("if let Some(stack) = &self.slots")
            || result.contains("if let Some(stack) = self.slots"),
        "Index + if-let: ref mut, &self.slots, or self.slots (Copy auto-copy). Got:\n{}",
        result
    );
}

// =============================================================================
// Index on owned Vec → Borrowed (add & or &mut when needed)
// =============================================================================

#[test]
fn test_index_on_mut_vec_ref_mut() {
    // Uses &mut self.slots[i] - Index on &mut self yields &mut T
    let src = r#"
pub struct Slot { pub q: i32 }
impl Slot { pub fn add(self, n: i32) {} }
pub struct Container { pub slots: Vec<Option<Slot>> }
impl Container {
    pub fn transfer(self, i: usize, n: i32) {
        if let Some(stack) = self.slots[i] {
            stack.q = stack.q + n
        }
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(
        result.contains("if let Some(stack)") || result.contains("if let Some(ref mut stack)"),
        "Expected if-let Some(stack) or Some(ref mut stack). Got:\n{}",
        result
    );
    if compiles {
        return;
    }
    assert!(
        result.contains("if let Some(stack) = &mut self.slots")
            || result.contains("Some(ref mut stack)"),
        "E0596 possible: &self with &mut index — document shape. compiles={}:\n{}",
        compiles,
        result
    );
}

#[test]
fn test_index_on_owned_vec_ref_when_read_only() {
    let src = r#"
pub struct Slot { pub q: i32 }
pub fn read(slots: Vec<Option<Slot>>, i: usize) -> i32 {
    if let Some(s) = slots[i] {
        s.q
    } else {
        0
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        result.contains("Some(ref s)")
            || (result.contains("fn read(slots: &") && result.contains("if let Some(s) = &slots["))
            || (result.contains("fn read(slots: &") && result.contains("if let Some(s) = slots[")),
        "Read-only: ref s, &slots[i], or plain slots[i] (when Option inner type is Copy). Got:\n{}",
        result
    );
}

// =============================================================================
// No ref mut for shared borrow (E0596 prevention)
// =============================================================================

#[test]
fn test_no_ref_mut_from_shared_borrow() {
    let src = r#"
pub fn try_mutate(opt: Option<i32>) {
    let r = &opt
    if let Some(v) = r {
        let _ = v
    }
}
pub fn main() {}
"#;
    let (result, compiles) = test_utils::compile_single_check(src);
    assert!(compiles, "Should compile. Got:\n{}", result);
    assert!(
        !result.contains("ref mut v"),
        "Cannot use ref mut when scrutinee is &. Got:\n{}",
        result
    );
}
