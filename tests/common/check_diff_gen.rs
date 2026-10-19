//! Seeded program generator for the check-vs-build gate: a valid base program (numbers and names
//! vary with the seed) and mutation operators, each a known-invalid edit marked `// MUTATED <op>`.

pub struct Operator {
    pub name: &'static str,
    /// `(slot, replacement)`: the base's line `// @slot` is replaced by the invalid statement(s).
    pub apply: fn(&mut Rng) -> (&'static str, String),
}

pub struct Rng(u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        // xorshift64*
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo) as u64) as i64
    }
    pub fn pick<'a>(&mut self, v: &[&'a str]) -> &'a str {
        v[(self.next() % v.len() as u64) as usize]
    }
}

const BASE: &str = r#"struct Point {
    x: int,
    y: int,
}

impl Point {
    fn sum(self) -> int {
        self.x + self.y
    }
}

enum Shape {
    Circle(int),
    Square(int),
}

fn area(s: Shape) -> int {
    match s {
        Shape::Circle(r) => r * r * 3,
        Shape::Square(a) => a * a,
    }
}

fn scale(v: int, k: int) -> int {
    v * k
}

fn describe(n: int) -> string {
    if n > 10 {
        "big"
    } else {
        "small"
    }
}

fn main() {
    let p = Point { x: @A, y: @B }
    let @NAME = scale(p.sum(), @C)
    let label = describe(@NAME)
    // @slot1
    if @NAME > @D {
        println("${label} ${@NAME}")
    } else {
        println("other ${area(Shape::Square(@A))}")
    }
    // @slot2
}
"#;

fn s(x: impl Into<String>) -> String {
    x.into()
}

pub const OPERATORS: &[Operator] = &[
    Operator {
        name: "string-into-int-annotation",
        apply: |r| ("slot1", format!("let bad: int = \"s{}\"", r.int(0, 99))),
    },
    Operator {
        name: "int-into-string-annotation",
        apply: |r| ("slot1", format!("let bad: string = {}", r.int(1, 99))),
    },
    Operator {
        name: "bool-into-int-annotation",
        apply: |r| {
            (
                "slot1",
                s(format!("let bad: int = {}", r.pick(&["true", "false"]))),
            )
        },
    },
    Operator {
        name: "undefined-variable",
        apply: |r| {
            (
                "slot2",
                format!(
                    "println(\"${{{}}}\")",
                    r.pick(&["missing_var", "nope", "undefined_x"])
                ),
            )
        },
    },
    Operator {
        name: "undefined-function",
        apply: |r| {
            (
                "slot2",
                format!(
                    "let bad = {}({})",
                    r.pick(&["no_such_fn", "frobnicate"]),
                    r.int(1, 9)
                ),
            )
        },
    },
    Operator {
        name: "wrong-arity",
        apply: |r| ("slot2", format!("let bad = scale({})", r.int(1, 9))),
    },
    Operator {
        name: "arg-type-mismatch",
        apply: |r| ("slot2", format!("let bad = scale(\"x{}\", 2)", r.int(1, 9))),
    },
    Operator {
        name: "unknown-method",
        apply: |r| {
            (
                "slot2",
                format!("let bad = p.{}()", r.pick(&["product", "norm", "missing"])),
            )
        },
    },
    Operator {
        name: "unknown-field",
        apply: |r| {
            (
                "slot2",
                format!("let bad = p.{}", r.pick(&["z", "w", "depth"])),
            )
        },
    },
    Operator {
        name: "missing-struct-field",
        apply: |r| ("slot2", format!("let bad = Point {{ x: {} }}", r.int(1, 9))),
    },
    Operator {
        name: "string-plus-int",
        apply: |r| ("slot2", format!("let bad = label + {}", r.int(1, 9))),
    },
    Operator {
        name: "non-bool-condition",
        apply: |r| {
            (
                "slot2",
                format!("if {} {{\n        println(\"x\")\n    }}", r.int(1, 9)),
            )
        },
    },
    Operator {
        name: "reassign-different-type",
        apply: |r| {
            (
                "slot2",
                format!("let mut bad = {}\n    bad = \"s\"", r.int(1, 9)),
            )
        },
    },
    Operator {
        name: "unknown-type-annotation",
        apply: |r| {
            (
                "slot2",
                format!(
                    "let bad: {} = {}",
                    r.pick(&["Widget", "Frob", "Nope"]),
                    r.int(1, 9)
                ),
            )
        },
    },
    Operator {
        name: "unknown-enum-variant",
        apply: |r| {
            (
                "slot2",
                format!(
                    "let bad = area(Shape::{}({}))",
                    r.pick(&["Triangle", "Hex"]),
                    r.int(1, 9)
                ),
            )
        },
    },
    Operator {
        name: "return-type-mismatch-call",
        apply: |r| ("slot2", format!("let bad: int = describe({})", r.int(1, 9))),
    },
];

/// The base program for `seed`, with `op` applied (its edit marked `// MUTATED <name>`).
pub fn instantiate(seed: u64, op: Option<&Operator>) -> String {
    let mut r = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
    let name = format!(
        "{}_{}",
        r.pick(&["total", "acc", "value", "score"]),
        r.int(0, 50)
    );
    let mut src = BASE
        .replace("@NAME", &name)
        .replace("@A", &r.int(1, 30).to_string())
        .replace("@B", &r.int(1, 30).to_string())
        .replace("@C", &r.int(2, 9).to_string())
        .replace("@D", &r.int(5, 200).to_string());
    if let Some(op) = op {
        let (slot, text) = (op.apply)(&mut r);
        let text = text.replace('\n', &format!(" // MUTATED {}\n", op.name));
        src = src.replace(
            &format!("// @{slot}"),
            &format!("{text} // MUTATED {}", op.name),
        );
    }
    src.lines()
        .filter(|l| !l.trim_start().starts_with("// @"))
        .map(|l| format!("{l}\n"))
        .collect()
}
