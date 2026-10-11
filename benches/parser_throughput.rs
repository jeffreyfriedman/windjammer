//! Parse throughput (tokens already lexed; measures `Parser::parse` + drop of the AST).
//! Run: `cargo bench --bench parser_throughput`. Record machine load next to any number you publish.
use std::time::Instant;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;

const SRC: &str = r#"
struct P { x: int, y: int }
impl P {
    fn sum(self) -> int {
        let a = self.x + self.y * 2
        match a {
            0 => 1,
            n if n > 3 => { let t = n * (n + 1); t }
            _ => if a > 1 { a } else { -a },
        }
    }
}
fn main() {
    let p = P { x: 1, y: 2 }
    let v = [1, 2, 3].len() + p.sum()
    println("v = ${v}")
}
"#;

fn parse_once(src: &str) -> usize {
    let tokens = Lexer::new(src).tokenize_with_locations();
    let mut parser = Parser::new_with_source(tokens, "b.wj".into(), src.into());
    let program = parser.parse().expect("parses");
    drop(parser);
    program.items.len()
}

fn main() {
    // A larger input: the sample repeated with distinct function names.
    let big: String = (0..200)
        .map(|i| SRC.replace("fn main", &format!("fn main{i}")).replace("struct P", &format!("struct P{i}")).replace("impl P", &format!("impl P{i}")).replace("let p = P {", &format!("let p = P{i} {{")))
        .collect();
    for (name, src, iters) in [("small", SRC.to_string(), 20000usize), ("big(200x)", big, 200)] {
        for _ in 0..iters / 10 {
            std::hint::black_box(parse_once(&src));
        }
        let mut best = f64::MAX;
        for _ in 0..5 {
            let t = Instant::now();
            for _ in 0..iters {
                std::hint::black_box(parse_once(&src));
            }
            best = best.min(t.elapsed().as_secs_f64());
        }
        let per = best / iters as f64;
        println!("parse[{name}]: {:.1} us/parse, {:.1} MB/s ({} bytes)", per * 1e6, src.len() as f64 / per / 1e6, src.len());
    }
}
