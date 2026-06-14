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

/// TDD Test: No .clone() on borrowed fields when used in borrow context (&expr)
///
/// Bug: `for ingredient in &recipe.ingredients.clone()` clones the entire Vec
/// just to iterate by reference. The `&` means we only need a reference, so
/// cloning is wasteful (O(n) allocation).
///
/// Root Cause: The borrowed iterator clone logic didn't check `in_borrow_context`.
/// When generating `&recipe.ingredients`, the `&` sets `in_borrow_context = true`,
/// but the FieldAccess handler didn't check this flag.
///
/// Fix: Added `!self.in_borrow_context` to the borrowed iterator clone condition.
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn test_no_clone_on_vec_field_in_for_loop_borrow() {
    // Pattern from windjammer-game rpg/crafting.wj:
    // for ingredient in &recipe.ingredients { ... }
    // recipe is borrowed — &recipe.ingredients is sufficient, no .clone() needed
    let source = r#"
pub struct Ingredient {
    pub name: string,
    pub amount: i32,
}

pub struct Recipe {
    pub ingredients: Vec<Ingredient>,
}

pub fn count_ingredients(recipes: Vec<Recipe>) -> i32 {
    let mut total = 0
    for recipe in &recipes {
        for ingredient in &recipe.ingredients {
            total = total + ingredient.amount
        }
    }
    total
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // Should NOT clone ingredients Vec just to iterate
    assert!(
        !generated.contains(".ingredients.clone()"),
        "Should not clone Vec field when iterating by reference.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_field_passed_by_ref() {
    // When passing a borrowed field by reference, no clone needed
    let source = r#"
pub struct Data {
    pub items: Vec<i32>,
}

pub fn process(data: Vec<Data>) -> i32 {
    let mut total = 0
    for d in &data {
        total = total + d.items.len() as i32
    }
    total
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // .len() takes &self, so no clone needed on items
    assert!(
        !generated.contains("d.items.clone()"),
        "Should not clone Vec field when calling .len() on it.\nGenerated:\n{}",
        generated
    );
}
