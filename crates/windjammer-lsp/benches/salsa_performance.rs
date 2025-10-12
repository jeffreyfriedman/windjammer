use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tower_lsp::lsp_types::Url;
use windjammer_lsp::database::WindjammerDatabase;

/// Small file (simple function)
const SMALL_FILE: &str = r#"
fn main() {
    println("Hello, world!");
}
"#;

/// Medium file (struct + impl + multiple functions)
const MEDIUM_FILE: &str = r#"
struct Point {
    x: int,
    y: int,
}

impl Point {
    fn new(x: int, y: int) -> Point {
        Point { x, y }
    }
    
    fn distance_from_origin(self) -> float {
        let dx = self.x as float;
        let dy = self.y as float;
        (dx * dx + dy * dy).sqrt()
    }
    
    fn translate(mut self, dx: int, dy: int) -> Point {
        self.x += dx;
        self.y += dy;
        self
    }
}

fn main() {
    let p = Point::new(3, 4);
    println("Distance: {}", p.distance_from_origin());
    let p2 = p.translate(1, 1);
    println("New point: ({}, {})", p2.x, p2.y);
}
"#;

/// Large file (multiple structs, traits, impls)
const LARGE_FILE: &str = r#"
trait Shape {
    fn area(self) -> float;
    fn perimeter(self) -> float;
}

struct Circle {
    radius: float,
}

impl Circle {
    fn new(radius: float) -> Circle {
        Circle { radius }
    }
}

impl Shape for Circle {
    fn area(self) -> float {
        3.14159 * self.radius * self.radius
    }
    
    fn perimeter(self) -> float {
        2.0 * 3.14159 * self.radius
    }
}

struct Rectangle {
    width: float,
    height: float,
}

impl Rectangle {
    fn new(width: float, height: float) -> Rectangle {
        Rectangle { width, height }
    }
    
    fn is_square(self) -> bool {
        self.width == self.height
    }
}

impl Shape for Rectangle {
    fn area(self) -> float {
        self.width * self.height
    }
    
    fn perimeter(self) -> float {
        2.0 * (self.width + self.height)
    }
}

struct Triangle {
    a: float,
    b: float,
    c: float,
}

impl Triangle {
    fn new(a: float, b: float, c: float) -> Triangle {
        Triangle { a, b, c }
    }
    
    fn is_valid(self) -> bool {
        self.a + self.b > self.c &&
        self.b + self.c > self.a &&
        self.c + self.a > self.b
    }
}

impl Shape for Triangle {
    fn area(self) -> float {
        let s = (self.a + self.b + self.c) / 2.0;
        (s * (s - self.a) * (s - self.b) * (s - self.c)).sqrt()
    }
    
    fn perimeter(self) -> float {
        self.a + self.b + self.c
    }
}

fn main() {
    let circle = Circle::new(5.0);
    println("Circle area: {}", circle.area());
    
    let rect = Rectangle::new(10.0, 20.0);
    println("Rectangle area: {}", rect.area());
    
    let tri = Triangle::new(3.0, 4.0, 5.0);
    if tri.is_valid() {
        println("Triangle area: {}", tri.area());
    }
}
"#;

fn bench_initial_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("initial_parse");

    // Small file
    group.bench_function(BenchmarkId::new("small", "first_parse"), |b| {
        b.iter(|| {
            let mut db = WindjammerDatabase::new();
            let uri = Url::parse("file:///test.wj").unwrap();
            let file = db.set_source_text(uri, black_box(SMALL_FILE.to_string()));
            black_box(db.get_program(file));
        });
    });

    // Medium file
    group.bench_function(BenchmarkId::new("medium", "first_parse"), |b| {
        b.iter(|| {
            let mut db = WindjammerDatabase::new();
            let uri = Url::parse("file:///test.wj").unwrap();
            let file = db.set_source_text(uri, black_box(MEDIUM_FILE.to_string()));
            black_box(db.get_program(file));
        });
    });

    // Large file
    group.bench_function(BenchmarkId::new("large", "first_parse"), |b| {
        b.iter(|| {
            let mut db = WindjammerDatabase::new();
            let uri = Url::parse("file:///test.wj").unwrap();
            let file = db.set_source_text(uri, black_box(LARGE_FILE.to_string()));
            black_box(db.get_program(file));
        });
    });

    group.finish();
}

fn bench_memoized_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("memoized_parse");

    // Small file - cached
    group.bench_function(BenchmarkId::new("small", "cached"), |b| {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, SMALL_FILE.to_string());
        // First parse to populate cache
        black_box(db.get_program(file));

        b.iter(|| {
            // This should hit the cache!
            black_box(db.get_program(file));
        });
    });

    // Medium file - cached
    group.bench_function(BenchmarkId::new("medium", "cached"), |b| {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, MEDIUM_FILE.to_string());
        black_box(db.get_program(file));

        b.iter(|| {
            black_box(db.get_program(file));
        });
    });

    // Large file - cached
    group.bench_function(BenchmarkId::new("large", "cached"), |b| {
        let mut db = WindjammerDatabase::new();
        let uri = Url::parse("file:///test.wj").unwrap();
        let file = db.set_source_text(uri, LARGE_FILE.to_string());
        black_box(db.get_program(file));

        b.iter(|| {
            black_box(db.get_program(file));
        });
    });

    group.finish();
}

fn bench_incremental_edit(c: &mut Criterion) {
    let mut group = c.benchmark_group("incremental_edit");

    // Small edit to medium file
    group.bench_function(BenchmarkId::new("medium", "small_edit"), |b| {
        b.iter_batched(
            || {
                let mut db = WindjammerDatabase::new();
                let uri = Url::parse("file:///test.wj").unwrap();
                let file = db.set_source_text(uri.clone(), MEDIUM_FILE.to_string());
                black_box(db.get_program(file)); // Initial parse
                (db, uri)
            },
            |(mut db, uri): (WindjammerDatabase, Url)| {
                // Make a small edit (change a number)
                let edited = MEDIUM_FILE.replace("3, 4", "5, 6");
                let file = db.set_source_text(uri, edited);
                black_box(db.get_program(file));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Add a new function to medium file
    group.bench_function(BenchmarkId::new("medium", "add_function"), |b| {
        b.iter_batched(
            || {
                let mut db = WindjammerDatabase::new();
                let uri = Url::parse("file:///test.wj").unwrap();
                let file = db.set_source_text(uri.clone(), MEDIUM_FILE.to_string());
                black_box(db.get_program(file));
                (db, uri)
            },
            |(mut db, uri): (WindjammerDatabase, Url)| {
                let edited = format!("{}\n\nfn helper() -> int {{ 42 }}", MEDIUM_FILE);
                let file = db.set_source_text(uri, edited);
                black_box(db.get_program(file));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_multiple_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("multiple_files");

    // Parse 3 files
    group.bench_function("parse_3_files", |b| {
        b.iter(|| {
            let mut db = WindjammerDatabase::new();

            let uri1 = Url::parse("file:///file1.wj").unwrap();
            let file1 = db.set_source_text(uri1, black_box(SMALL_FILE.to_string()));
            black_box(db.get_program(file1));

            let uri2 = Url::parse("file:///file2.wj").unwrap();
            let file2 = db.set_source_text(uri2, black_box(MEDIUM_FILE.to_string()));
            black_box(db.get_program(file2));

            let uri3 = Url::parse("file:///file3.wj").unwrap();
            let file3 = db.set_source_text(uri3, black_box(LARGE_FILE.to_string()));
            black_box(db.get_program(file3));
        });
    });

    // Re-query all 3 files (should be cached)
    group.bench_function("requery_3_files", |b| {
        let mut db = WindjammerDatabase::new();

        let uri1 = Url::parse("file:///file1.wj").unwrap();
        let file1 = db.set_source_text(uri1, SMALL_FILE.to_string());
        black_box(db.get_program(file1));

        let uri2 = Url::parse("file:///file2.wj").unwrap();
        let file2 = db.set_source_text(uri2, MEDIUM_FILE.to_string());
        black_box(db.get_program(file2));

        let uri3 = Url::parse("file:///file3.wj").unwrap();
        let file3 = db.set_source_text(uri3, LARGE_FILE.to_string());
        black_box(db.get_program(file3));

        b.iter(|| {
            black_box(db.get_program(file1));
            black_box(db.get_program(file2));
            black_box(db.get_program(file3));
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_initial_parse,
    bench_memoized_parse,
    bench_incremental_edit,
    bench_multiple_files
);
criterion_main!(benches);
