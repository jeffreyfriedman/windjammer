#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

/// TDD: if-let binding reassigned to `let mut` then mutated should compile.
///
/// Bug (E0596): When an if-let binding is assigned to a new `let mut` variable
/// and then mutated, the compiler generates code that leaves the binding as a
/// shared reference. The mutation detection only checks for DIRECT mutation on
/// the pattern binding, not transitive mutation through reassignment.
///
/// Pattern:
///   if let Some(stack) = self.slots[i] {
///       let mut new_stack = stack
///       new_stack.add(quantity)  // E0596: cannot borrow as mutable
///   }
///
/// Root cause: `binding_receives_mutating_call_with_sig_check` only looks for
/// `stack.add()` or `stack.field = ...`, not `let mut x = stack; x.add()`.
/// When mutation is not detected:
///   1. `effective_option_scrutinee_ref_prefix` strips `&mut` prefix
///   2. Clone is applied to produce `&self.slots[i].clone()`
///   3. Match ergonomics bind `stack` as `&ItemStack`
///   4. `let mut new_stack = stack` doesn't clone (type looks owned)
///   5. `new_stack.add()` fails with E0596
///
/// Fix: When a binding is reassigned to `let mut x = binding`, clone at the
/// assignment site to produce an owned value.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_if_let_binding_reassigned_to_mut_then_mutated() {
    let generated = test_utils::compile_single(
        r#"
pub struct ItemStack {
    pub item_id: int
    pub quantity: int
}

impl ItemStack {
    pub fn add(self, amount: int) {
        self.quantity = self.quantity + amount
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>
}

impl Inventory {
    pub fn add_to_slot(self, index: int, amount: int) {
        if let Some(stack) = self.slots[index] {
            let mut new_stack = stack
            new_stack.add(amount)
        }
    }
}
"#,
    );

    // The generated code must compile without E0596.
    // Either:
    // 1. The binding `stack` should be owned (via clone on the scrutinee), OR
    // 2. `let mut new_stack = stack` should clone the reference
    let has_clone_on_assignment = generated.contains("stack.clone()")
        || generated.contains("new_stack = stack.clone()");
    let has_ref_mut = generated.contains("ref mut stack")
        || generated.contains("&mut self.slots");
    let has_owned_binding = !generated.contains("&self.slots[")
        || has_clone_on_assignment;

    assert!(
        has_clone_on_assignment || has_ref_mut || has_owned_binding,
        "Generated code should handle ownership correctly for reassigned if-let binding.\n\
         Expected: clone on assignment, ref mut binding, or owned scrutinee.\n\
         Generated:\n{}",
        generated
    );
}

#[test]
fn test_if_let_direct_mutation_still_works() {
    let generated = test_utils::compile_single(
        r#"
pub struct ItemStack {
    pub item_id: int
    pub quantity: int
}

impl ItemStack {
    pub fn add(self, amount: int) {
        self.quantity = self.quantity + amount
    }
}

pub struct Inventory {
    pub slots: Vec<Option<ItemStack>>
}

impl Inventory {
    pub fn modify_slot(self, index: int, amount: int) {
        if let Some(stack) = self.slots[index] {
            stack.add(amount)
        }
    }
}
"#,
    );

    // Direct mutation should use ref mut or &mut
    assert!(
        generated.contains("ref mut") || generated.contains("&mut"),
        "Direct mutation of if-let binding should use ref mut or &mut.\nGenerated:\n{}",
        generated
    );
}
