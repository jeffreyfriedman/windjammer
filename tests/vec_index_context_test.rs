/// TDD Test: Vec index auto-clone must be suppressed in certain contexts
///
/// Bug: The compiler adds .clone() to Vec<T>[i] for non-Copy types, but
/// this is incorrect in three contexts:
///
/// 1. Assignment target: `self.vec[i] = value` → must NOT clone (can't assign to .clone())
/// 2. Borrow context: `&self.vec[i]` → must NOT clone (want reference to original)
/// 3. Mutable borrow context: `&mut self.vec[i]` → must NOT clone (can't take &mut of temp)
///
/// Root cause: The Expression::Index handler's auto-clone fallback doesn't check
/// whether we're generating an assignment target or inside a reference expression.
///
/// Discovered via dogfooding: ecs/components.wj (ComponentArray<T>)
#[path = "test_utils.rs"]
mod test_utils;

#[test]
fn test_vec_index_no_clone_on_assignment_target() {
    // Bug: `self.items[i] = value` generates `self.items[i as usize].clone() = value`
    // The .clone() on the LEFT side of assignment is always wrong.
    let source = r#"
struct Item {
    name: string,
    count: i32,
}

struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: vec![] }
    }

    pub fn update_at(&mut self, index: i32, item: Item) {
        self.items[index as usize] = item
    }
}

fn main() {
    let mut c = Container::new()
    c.items.push(Item { name: "old".to_string(), count: 0 })
    c.update_at(0, Item { name: "new".to_string(), count: 1 })
    println("done")
}
"#;

    let (rust_code, compiles) = test_utils::compile_single_check(source);

    // The assignment target must NOT have .clone()
    // Bad: self.items[index as usize].clone() = item
    // Good: self.items[index as usize] = item
    assert!(
        !rust_code.contains(".clone() = "),
        "Assignment target must NOT have .clone()!\nGenerated:\n{}",
        rust_code
    );

    assert!(
        compiles,
        "Generated Rust must compile!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_index_no_clone_on_borrow() {
    // Bug: `&self.items[i]` generates `&self.items[i as usize].clone()`
    // Taking a reference to a clone is pointless and wrong (reference to temporary).
    let source = r#"
struct Item {
    name: string,
    count: i32,
}

struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: vec![] }
    }

    pub fn get_ref(&self, index: i32) -> &Item {
        &self.items[index as usize]
    }
}

fn main() {
    let c = Container::new()
    println("done")
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Borrow context must NOT have .clone()
    // Bad: &self.items[index as usize].clone()
    // Good: &self.items[index as usize]
    assert!(
        !rust_code.contains(".clone()"),
        "Borrow of Vec index must NOT have .clone()!\nGenerated:\n{}",
        rust_code
    );
}

#[test]
fn test_vec_index_no_clone_on_mut_borrow() {
    // Bug: `&mut self.items[i]` generates `&mut self.items[i as usize].clone()`
    // Can't take &mut of a temporary.
    let source = r#"
struct Item {
    name: string,
    count: i32,
}

struct Container {
    items: Vec<Item>,
}

impl Container {
    pub fn new() -> Container {
        Container { items: vec![] }
    }

    pub fn get_mut_ref(&mut self, index: i32) -> &mut Item {
        &mut self.items[index as usize]
    }
}

fn main() {
    let mut c = Container::new()
    println("done")
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Mutable borrow context must NOT have .clone()
    // Bad: &mut self.items[index as usize].clone()
    // Good: &mut self.items[index as usize]
    assert!(
        !rust_code.contains(".clone()"),
        "Mutable borrow of Vec index must NOT have .clone()!\nGenerated:\n{}",
        rust_code
    );
}
