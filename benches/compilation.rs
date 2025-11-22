use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use windjammer::{analyzer, codegen, lexer, parser, CompilationTarget};

// Sample Windjammer programs of varying complexity
const SIMPLE_PROGRAM: &str = r#"
fn add(x: int, y: int) -> int {
    x + y
}

fn main() {
    let result = add(40, 2)
    println!("{}", result)
}
"#;

const MEDIUM_PROGRAM: &str = r#"
struct Point {
    x: float,
    y: float
}

impl Point {
    fn new(x: float, y: float) -> Point {
        Point { x, y }
    }
    
    fn distance(&self, other: &Point) -> float {
        let dx = self.x - other.x
        let dy = self.y - other.y
        (dx * dx + dy * dy).sqrt()
    }
}

fn main() {
    let p1 = Point::new(0.0, 0.0)
    let p2 = Point::new(3.0, 4.0)
    let dist = p1.distance(&p2)
    println!("Distance: {}", dist)
}
"#;

const COMPLEX_PROGRAM: &str = r#"
trait Display {
    fn display(&self) -> string {
        "default"
    }
}

struct User {
    name: string,
    age: int,
    email: string
}

impl Display for User {
    fn display(&self) -> string {
        "User: ${self.name}, age ${self.age}"
    }
}

impl User {
    fn new(name: string, age: int, email: string) -> User {
        User { 
            name: name,
            age: age,
            email: email
        }
    }
    
    fn is_adult(&self) -> bool {
        self.age >= 18
    }
    
    fn get_age(&self) -> int {
        self.age
    }
}

fn count_adults(user1: &User, user2: &User, user3: &User) -> int {
    let mut total = 0
    if user1.is_adult() {
        total = total + 1
    }
    if user2.is_adult() {
        total = total + 1
    }
    if user3.is_adult() {
        total = total + 1
    }
    total
}

fn main() {
    let user1 = User::new("Alice", 25, "alice@example.com")
    let user2 = User::new("Bob", 17, "bob@example.com")
    let user3 = User::new("Charlie", 30, "charlie@example.com")
    
    let adults = count_adults(&user1, &user2, &user3)
    println!("Adults: {}", adults)
    
    println!("{}", user1.display())
    println!("{}", user2.display())
    println!("{}", user3.display())
}
"#;

fn benchmark_lexer(c: &mut Criterion) {
    let mut group = c.benchmark_group("lexer");

    group.bench_with_input(
        BenchmarkId::new("simple", "10_lines"),
        &SIMPLE_PROGRAM,
        |b, source| {
            b.iter(|| {
                let mut lexer = lexer::Lexer::new(black_box(source));
                lexer.tokenize_with_locations()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("medium", "30_lines"),
        &MEDIUM_PROGRAM,
        |b, source| {
            b.iter(|| {
                let mut lexer = lexer::Lexer::new(black_box(source));
                lexer.tokenize_with_locations()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("complex", "50_lines"),
        &COMPLEX_PROGRAM,
        |b, source| {
            b.iter(|| {
                let mut lexer = lexer::Lexer::new(black_box(source));
                lexer.tokenize_with_locations()
            });
        },
    );

    group.finish();
}

fn benchmark_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");

    // Pre-lex the programs
    let simple_tokens = {
        let mut lexer = lexer::Lexer::new(SIMPLE_PROGRAM);
        lexer.tokenize_with_locations()
    };

    let medium_tokens = {
        let mut lexer = lexer::Lexer::new(MEDIUM_PROGRAM);
        lexer.tokenize_with_locations()
    };

    let complex_tokens = {
        let mut lexer = lexer::Lexer::new(COMPLEX_PROGRAM);
        lexer.tokenize_with_locations()
    };

    group.bench_with_input(
        BenchmarkId::new("simple", "10_lines"),
        &simple_tokens,
        |b, tokens| {
            b.iter(|| {
                let mut parser = parser::Parser::new(black_box(tokens.clone()));
                parser.parse().unwrap()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("medium", "30_lines"),
        &medium_tokens,
        |b, tokens| {
            b.iter(|| {
                let mut parser = parser::Parser::new(black_box(tokens.clone()));
                parser.parse().unwrap()
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("complex", "50_lines"),
        &complex_tokens,
        |b, tokens| {
            b.iter(|| {
                let mut parser = parser::Parser::new(black_box(tokens.clone()));
                parser.parse().unwrap()
            });
        },
    );

    group.finish();
}

fn benchmark_full_compilation(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_compilation");

    group.bench_with_input(
        BenchmarkId::new("simple", "10_lines"),
        &SIMPLE_PROGRAM,
        |b, source| {
            b.iter(|| {
                // Lex
                let mut lexer = lexer::Lexer::new(black_box(source));
                let tokens = lexer.tokenize_with_locations();

                // Parse
                let mut parser = parser::Parser::new(tokens);
                let program = parser.parse().unwrap();

                // Analyze
                let mut analyzer = analyzer::Analyzer::new();
                let (analyzed, signatures) = analyzer.analyze_program(&program).unwrap();

                // Generate
                let mut generator =
                    codegen::CodeGenerator::new(signatures, CompilationTarget::Wasm);
                generator.generate_program(&program, &analyzed)
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("medium", "30_lines"),
        &MEDIUM_PROGRAM,
        |b, source| {
            b.iter(|| {
                let mut lexer = lexer::Lexer::new(black_box(source));
                let tokens = lexer.tokenize_with_locations();
                let mut parser = parser::Parser::new(tokens);
                let program = parser.parse().unwrap();
                let mut analyzer = analyzer::Analyzer::new();
                let (analyzed, signatures) = analyzer.analyze_program(&program).unwrap();
                let mut generator =
                    codegen::CodeGenerator::new(signatures, CompilationTarget::Wasm);
                generator.generate_program(&program, &analyzed)
            });
        },
    );

    group.bench_with_input(
        BenchmarkId::new("complex", "50_lines"),
        &COMPLEX_PROGRAM,
        |b, source| {
            b.iter(|| {
                let mut lexer = lexer::Lexer::new(black_box(source));
                let tokens = lexer.tokenize_with_locations();
                let mut parser = parser::Parser::new(tokens);
                let program = parser.parse().unwrap();
                let mut analyzer = analyzer::Analyzer::new();
                let (analyzed, signatures) = analyzer.analyze_program(&program).unwrap();
                let mut generator =
                    codegen::CodeGenerator::new(signatures, CompilationTarget::Wasm);
                generator.generate_program(&program, &analyzed)
            });
        },
    );

    group.finish();
}

criterion_group!(
    benches,
    benchmark_lexer,
    benchmark_parser,
    benchmark_full_compilation
);
criterion_main!(benches);
