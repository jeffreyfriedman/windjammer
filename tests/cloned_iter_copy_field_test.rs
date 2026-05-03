/// TDD Test: No .clone() on Copy fields when iterating a CLONED collection
///
/// Bug: When iterating `for ingredient in &recipe.ingredients.clone()`,
/// `ingredient` is a borrowed variable. Accessing `ingredient.quantity` (i32, Copy)
/// incorrectly gets `.clone()` because:
///   1. `infer_expression_type` couldn't resolve `.clone()` method return type
///   2. The for-loop variable `ingredient` had unknown type
///   3. `ingredient.quantity`'s type couldn't be resolved as i32 (Copy)
///   4. Name heuristic missed "quantity"
///
/// Root Cause: `infer_expression_type` for MethodCall didn't handle `.clone()`
/// (which returns the same type as its receiver). This broke the type inference
/// chain: collection.clone() → Vec<T> → &Vec<T> → element: T → T.field: i32.
///
/// Fix: Added `.clone()` handler in `infer_expression_type` MethodCall case.
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_no_clone_on_i32_field_via_cloned_iter() {
    // Pattern from windjammer-game rpg/crafting.wj:
    // for ingredient in &recipe.ingredients.clone() { ingredient.quantity }
    // ingredient.quantity is i32 (Copy) — should NOT get .clone()
    let source = r#"
pub struct RecipeIngredient {
    pub item_id: string,
    pub quantity: i32,
}

pub struct Recipe {
    pub name: string,
    pub ingredients: Vec<RecipeIngredient>,
    pub gold_cost: i32,
}

impl Recipe {
    pub fn total_quantity(self) -> i32 {
        let mut total = 0
        for ingredient in &self.ingredients.clone() {
            total = total + ingredient.quantity
        }
        total
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // ingredient.quantity is i32 (Copy) — should NOT be cloned
    assert!(
        !generated.contains("ingredient.quantity.clone()"),
        "Should not clone i32 field 'quantity' via cloned iterable.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_no_clone_on_i32_field_gold_cost_via_match() {
    // Pattern from windjammer-game: recipe.gold_cost where recipe
    // is bound via match (if let Some(recipe) = self.recipes.get(...))
    let source = r#"
pub struct RecipeIngredient {
    pub item_id: string,
    pub quantity: i32,
}

pub struct Recipe {
    pub name: string,
    pub ingredients: Vec<RecipeIngredient>,
    pub gold_cost: i32,
}

pub struct CraftingManager {
    pub recipes: Vec<Recipe>,
}

impl CraftingManager {
    pub fn total_cost(self) -> i32 {
        let mut total = 0
        for recipe in &self.recipes {
            total = total + recipe.gold_cost
        }
        total
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // recipe.gold_cost is i32 (Copy) — should NOT be cloned
    assert!(
        !generated.contains("recipe.gold_cost.clone()"),
        "Should not clone i32 field 'gold_cost' via borrowed iter.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_clone_on_string_field_via_cloned_iter_is_ok() {
    // String field through cloned iterable SHOULD still get .clone()
    let source = r#"
pub struct RecipeIngredient {
    pub item_id: string,
    pub quantity: i32,
}

pub struct Recipe {
    pub name: string,
    pub ingredients: Vec<RecipeIngredient>,
}

impl Recipe {
    pub fn collect_ids(self) -> Vec<string> {
        let mut ids: Vec<string> = Vec::new()
        for ingredient in &self.ingredients.clone() {
            ids.push(ingredient.item_id.clone())
        }
        ids
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // item_id is String (NOT Copy) — clone IS expected
    assert!(
        generated.contains(".clone()"),
        "String field should still use .clone() via cloned iterable.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn test_nested_field_no_intermediate_clone_via_borrowed_iter() {
    // Pattern from windjammer-game: stack.item.stats.armor
    // where stack comes from `for stack in &self.slots`
    // The intermediate `item` should NOT be cloned if we're accessing
    // a Copy sub-field (stats.armor where armor is i32)
    let source = r#"
pub struct ItemStats {
    pub armor: i32,
    pub damage: i32,
}

pub struct Item {
    pub name: string,
    pub stats: ItemStats,
}

pub struct ItemStack {
    pub item: Item,
    pub quantity: i32,
}

pub struct Equipment {
    pub slots: Vec<ItemStack>,
}

impl Equipment {
    pub fn total_armor(self) -> i32 {
        let mut total = 0
        for stack in &self.slots {
            total = total + stack.item.stats.armor
        }
        total
    }
}
"#;

    let generated = test_utils::compile_single(source);
    println!("Generated:\n{}", generated);

    // stack.item should NOT have .clone() when we're reading stack.item.stats.armor (Copy)
    assert!(
        !generated.contains("stack.item.clone()"),
        "Should not clone intermediate 'item' when reading nested Copy field.\nGenerated:\n{}",
        generated
    );
}
