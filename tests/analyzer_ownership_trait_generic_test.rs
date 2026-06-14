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
#![allow(unused)]
//! Analyzer ownership: traits, generics, regressions.
#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// TRAIT IMPLEMENTATION SCENARIOS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_impl_preserves_signature() {
    let code = r#"
trait Printable {
    fn print(self) { }
}

@derive(Clone, Debug)
pub struct MyType {
    value: i32,
}

impl Printable for MyType {
    fn print(self) {
        println!("{}", self.value)
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Trait impl should match trait signature exactly
    assert!(success, "Error: {}", err);
}

// ============================================================================
// GENERIC TYPE SCENARIOS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_param_ownership() {
    let code = r#"
@derive(Clone, Debug)
pub struct Container<T> {
    value: T,
}

pub fn get_value<T: Clone>(c: Container<T>) -> T {
    c.value
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Generic containers should have sensible inference
    assert!(success, "Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_generic_with_clone() {
    let code = r#"
pub fn clone_item<T: Clone>(item: T) -> T {
    item
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Should work with T: Clone bound
    assert!(success, "Error: {}", err);
}

// ============================================================================
// BUG: Auto-Copy struct inference gap (E0382 in dogfooding)
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_auto_copy_struct_self_borrow_inference() {
    // Bug: When a struct has only Copy fields and no @derive decorator,
    // the codegen correctly auto-derives Copy on the struct, but the analyzer
    // doesn't know the return type is Copy, so it infers owned `self` instead
    // of `&self` for getter methods that return the struct type.
    //
    // This causes E0382 "use of moved value" when calling .id() and then
    // using the original value again:
    //   let id = thing.id();  // moves `thing`
    //   map.insert(id, thing); // ERROR: thing already moved
    let code = r#"
struct ThingId {
    value: u32
}

impl ThingId {
    pub fn new(value: u32) -> ThingId {
        ThingId { value: value }
    }
}

struct Thing {
    id: ThingId,
    name: string
}

impl Thing {
    // Test struct literal &string -> string conversion
    pub fn new(id: u32, name: string) -> Thing {
        Thing { id: ThingId::new(id), name: name }
    }

    pub fn id(self) -> ThingId {
        self.id
    }

    pub fn name(self) -> string {
        self.name
    }
}

fn main() {
    let thing = Thing::new(1, "test")
    let id = thing.id()
    println("{}", thing.name())
}
"#;
    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Codegen failed: {:?}", result.err());
    let generated = result.unwrap();

    // The ThingId struct should auto-derive Copy (all fields are Copy)
    // Therefore Thing::id() should get &self (since it returns a Copy type)
    assert!(
        generated.contains("fn id(&self)"),
        "id() should infer &self since ThingId is auto-Copy. Got:\n{}",
        generated
    );

    // Stored into a `string` field: API uses &str; struct literal coerces to String at the site.
    assert!(
        generated.contains("fn new(id: u32, name: &str)"),
        "name parameter stored in struct should infer &str at API. Got:\n{}",
        generated
    );
    assert!(
        generated.contains("name: name.to_string()")
            || generated.contains("name.to_string()")
            || generated.contains("name: name.into()")
            || generated.contains("name.into()")
            || generated.contains("name: name")
            || generated.contains(", name }")
            || generated.contains(", name,"),
        "struct literal should assign String name field. Got:\n{}",
        generated
    );

    // Verify the full program compiles (no E0382 "use of moved value")
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Should compile without E0382. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_usize_field_comparison_no_cast_mismatch() {
    // Bug: When a generic struct has usize fields and a method compares .len() with
    // a usize field, the codegen casts .len() to i64 but leaves the field as usize.
    // Both sides are usize so NO cast should be applied.
    //
    // Example: `self.available.len() == self.capacity` should NOT become
    // `(self.available.len() as i64) == self.capacity` (mismatched types)
    let code = r#"
struct Pool<T> {
    items: Vec<T>,
    capacity: usize,
    count: usize
}

impl<T> Pool<T> {
    pub fn new(cap: usize) -> Pool<T> {
        Pool { items: Vec::new(), capacity: cap, count: 0 }
    }

    pub fn is_full(self) -> bool {
        self.items.len() == self.capacity
    }

    pub fn has_space(self) -> bool {
        self.items.len() < self.capacity
    }
}

fn main() {
    let pool: Pool<int> = Pool::new(10 as usize)
    println("{}", pool.is_full())
    println("{}", pool.has_space())
}
"#;
    let result = test_utils::compile_single_result(code);
    assert!(result.is_ok(), "Codegen failed: {:?}", result.err());
    let generated = result.unwrap();

    // Both .len() and self.capacity are usize - no cast should be applied
    // Bad: (self.items.len() as i64) == self.capacity
    // Good: self.items.len() == self.capacity
    assert!(
        !generated.contains("as i64) == self.capacity"),
        "Should NOT cast .len() to i64 when comparing with usize field. Got:\n{}",
        generated
    );

    // Verify the full program compiles (no E0308 "mismatched types")
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Should compile without E0308. Error: {}", err);
}
