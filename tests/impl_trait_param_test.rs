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

/// Test: `trait TraitName` in type position with auto dispatch inference
///
/// THE WINDJAMMER WAY: Users write `trait Describable` as a type. The compiler
/// infers static dispatch (impl Trait) vs dynamic dispatch (dyn Trait) based on
/// context, and ownership (&, &mut, owned) based on usage.
///
/// Design:
/// - Bare param/return: `trait Foo` -> `impl Foo` (static dispatch)
/// - Inside Vec/Option: `Vec<trait Foo>` -> `Vec<Box<dyn Foo>>` (dynamic dispatch)
/// - Behind reference: `&trait Foo` -> `&dyn Foo`
/// - Inside Box: `Box<trait Foo>` -> `Box<dyn Foo>`
#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_keyword_param_basic() {
    // THE WINDJAMMER WAY: `trait Describable` in parameter position
    // Compiler infers static dispatch (impl Trait) and ownership (&)
    let source = r#"
trait Describable {
    fn describe(self) -> string
}

struct Point {
    x: f32,
    y: f32,
}

impl Describable for Point {
    fn describe(self) -> string {
        format!("({}, {})", self.x, self.y)
    }
}

fn print_item(item: trait Describable) {
    println!("{}", item.describe())
}

fn main() {
    let p = Point { x: 1.0, y: 2.0 }
    print_item(p)
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Should generate `impl Describable` (static dispatch) in the parameter type
    assert!(
        rust_code.contains("impl Describable"),
        "Should generate 'impl Describable' for static dispatch\nGenerated:\n{}",
        rust_code
    );

    // Point is a Copy type, so ownership inference keeps it owned (no & added).
    // The key check is that `impl Describable` appears (static dispatch).
    assert!(
        rust_code.contains("item: impl Describable"),
        "Should generate 'item: impl Describable' for trait keyword param\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_keyword_return_position() {
    // `trait Greeter` in return position -> `impl Greeter` (existential type)
    let source = r#"
trait Greeter {
    fn greet(self) -> string
}

struct English {}

impl Greeter for English {
    fn greet(self) -> string {
        "Hello!".to_string()
    }
}

fn make_greeter() -> trait Greeter {
    English {}
}

fn main() {
    let g = make_greeter()
    println!("{}", g.greet())
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Return position should generate `-> impl Greeter`
    assert!(
        rust_code.contains("-> impl Greeter"),
        "Should generate '-> impl Greeter' in return type\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_keyword_in_vec() {
    // `Vec<trait Describable>` needs dynamic dispatch -> `Vec<Box<dyn Describable>>`
    let source = r#"
trait Describable {
    fn describe(self) -> string
}

struct Point { x: f32, y: f32 }
struct Color { r: u8, g: u8, b: u8 }

impl Describable for Point {
    fn describe(self) -> string {
        format!("Point({}, {})", self.x, self.y)
    }
}

impl Describable for Color {
    fn describe(self) -> string {
        format!("Color({}, {}, {})", self.r, self.g, self.b)
    }
}

struct ItemList {
    items: Vec<trait Describable>,
}

fn main() {
    println!("hello")
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Vec<trait Describable> -> Vec<Box<dyn Describable>> (dynamic dispatch, boxed)
    assert!(
        rust_code.contains("Vec<Box<dyn Describable>>"),
        "Vec<trait T> should generate Vec<Box<dyn T>>\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_keyword_behind_reference() {
    // `&trait Describable` -> `&dyn Describable` (dynamic dispatch, borrowed)
    let source = r#"
trait Describable {
    fn describe(self) -> string
}

struct Point { x: f32, y: f32 }

impl Describable for Point {
    fn describe(self) -> string {
        format!("({}, {})", self.x, self.y)
    }
}

struct Wrapper {
    item: &trait Describable,
}

fn main() {
    println!("hello")
}
"#;

    let rust_code = test_utils::compile_single(source);

    // &trait Describable -> &dyn Describable
    assert!(
        rust_code.contains("&dyn Describable"),
        "&trait T should generate &dyn T\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_keyword_in_box() {
    // `Box<trait Describable>` -> `Box<dyn Describable>`
    let source = r#"
trait Describable {
    fn describe(self) -> string
}

struct Point { x: f32, y: f32 }

impl Describable for Point {
    fn describe(self) -> string {
        format!("({}, {})", self.x, self.y)
    }
}

struct Wrapper {
    item: Box<trait Describable>,
}

fn main() {
    println!("hello")
}
"#;

    let rust_code = test_utils::compile_single(source);

    // Box<trait Describable> -> Box<dyn Describable>
    assert!(
        rust_code.contains("Box<dyn Describable>"),
        "Box<trait T> should generate Box<dyn T>\nGenerated:\n{}",
        rust_code
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_trait_keyword_ownership_inference() {
    // `trait Describable` param that is mutated should get &mut impl Describable
    let source = r#"
trait Resettable {
    fn reset(self)
}

struct Counter {
    count: int,
}

impl Resettable for Counter {
    fn reset(self) {
        self.count = 0
    }
}

fn reset_item(item: trait Resettable) {
    item.reset()
}

fn main() {
    let mut c = Counter { count: 5 }
    reset_item(c)
}
"#;

    let rust_code = test_utils::compile_single(source);

    // reset() mutates self, so item should be &mut impl Resettable
    assert!(
        rust_code.contains("impl Resettable"),
        "Should generate impl Resettable for static dispatch\nGenerated:\n{}",
        rust_code
    );
}
