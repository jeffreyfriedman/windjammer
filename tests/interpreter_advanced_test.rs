/// Windjammerscript Interpreter Advanced Tests (TDD)
///
/// Tests for features discovered missing by running conformance .wj files
/// through the interpreter. Each test targets a specific bug.

/// Parse source and run through the interpreter, capturing output
fn interpret(source: &str) -> Result<String, String> {
    let mut lexer = windjammer::lexer::Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = windjammer::parser::Parser::new_with_source(
        tokens,
        "test.wj".to_string(),
        source.to_string(),
    );
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    let mut interp = windjammer::interpreter::Interpreter::new_capturing();
    interp.run(&program)?;
    Ok(interp.get_output())
}

// ==========================================
// Bug 1: String interpolation with ${}
// ==========================================

#[test]
fn test_interp_string_interpolation_dollar() {
    let output = interpret(
        r#"
fn main() {
    let name = "Alice"
    let age = 30
    println("[test] name=${name}, age=${age}")
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "[test] name=Alice, age=30");
}

// ==========================================
// Bug 2: Enum construction (Color::Red, etc.)
// ==========================================

#[test]
fn test_interp_enum_unit_variant() {
    let output = interpret(
        r#"
enum Color {
    Red,
    Green,
    Blue,
}

fn color_name(c: Color) -> string {
    match c {
        Color::Red => "red",
        Color::Green => "green",
        Color::Blue => "blue",
    }
}

fn main() {
    println("{}", color_name(Color::Red))
    println("{}", color_name(Color::Green))
}
"#,
    )
    .unwrap();
    assert!(output.contains("red"), "Expected 'red', got:\n{}", output);
    assert!(output.contains("green"), "Expected 'green', got:\n{}", output);
}

#[test]
fn test_interp_enum_tuple_variant() {
    let output = interpret(
        r#"
enum Shape {
    Circle(int),
    Square(int),
    Point,
}

fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => 3 * r * r,
        Shape::Square(side) => side * side,
        Shape::Point => 0,
    }
}

fn main() {
    println("{}", area(Shape::Circle(5)))
    println("{}", area(Shape::Square(4)))
    println("{}", area(Shape::Point))
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "75\n16\n0");
}

// ==========================================
// Bug 3: Vec macro and methods
// ==========================================

#[test]
fn test_interp_vec_macro() {
    let output = interpret(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    println("{}", items.len())
    println("{}", items[0])
    println("{}", items[2])
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "3\n10\n30");
}

#[test]
fn test_interp_vec_push_local() {
    let output = interpret(
        r#"
fn main() {
    let mut items = vec![1, 2]
    items.push(3)
    println("{}", items.len())
    println("{}", items[2])
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "3\n3");
}

// ==========================================
// Bug 4: Match as let expression
// ==========================================

#[test]
fn test_interp_match_let_expression() {
    let output = interpret(
        r#"
fn main() {
    let x = 5
    let label = match x {
        1 => "one",
        5 => "five",
        _ => "other",
    }
    println("{}", label)
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "five");
}

// ==========================================
// Bug 5: Enum methods (match self in impl)
// ==========================================

#[test]
fn test_interp_enum_method() {
    let output = interpret(
        r#"
enum Shape {
    Circle(int),
    Square(int),
    Point,
}

impl Shape {
    fn area(self) -> int {
        match self {
            Shape::Circle(r) => 3 * r * r,
            Shape::Square(s) => s * s,
            Shape::Point => 0,
        }
    }
}

fn main() {
    let c = Shape::Circle(5)
    println("{}", c.area())
    let s = Shape::Square(4)
    println("{}", s.area())
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "75\n16");
}

// ==========================================
// Bug 6: Static method constructors (Type::new)
// ==========================================

#[test]
fn test_interp_static_constructor() {
    let output = interpret(
        r#"
struct Player {
    hp: int,
}

impl Player {
    fn new(hp: int) -> Player {
        Player { hp }
    }

    fn get_hp(self) -> int {
        self.hp
    }
}

fn main() {
    let p = Player::new(100)
    println("{}", p.get_hp())
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "100");
}

// ==========================================
// Bug 7: For loop over vec![] items
// ==========================================

#[test]
fn test_interp_for_over_vec() {
    let output = interpret(
        r#"
fn main() {
    let items = vec![10, 20, 30]
    let mut sum = 0
    for item in items {
        sum += item
    }
    println("{}", sum)
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "60");
}

// ==========================================
// Bug 8: Struct shorthand field init
// ==========================================

#[test]
fn test_interp_struct_shorthand() {
    let output = interpret(
        r#"
struct Point {
    x: int,
    y: int,
}

fn main() {
    let x = 10
    let y = 20
    let p = Point { x, y }
    println("{} {}", p.x, p.y)
}
"#,
    )
    .unwrap();
    assert_eq!(output.trim(), "10 20");
}
