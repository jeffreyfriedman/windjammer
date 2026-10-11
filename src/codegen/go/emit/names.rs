//! Go identifier mangling.

const RESERVED: &[&str] = &[
    "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough", "for", "func", "go", "goto", "if", "import", "interface", "map", "package", "range", "return", "select", "struct", "switch", "type", "var",
    // predeclared identifiers that user code could shadow harmfully
    "bool", "byte", "complex64", "complex128", "error", "float32", "float64", "int", "int8", "int16", "int32", "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64", "uintptr", "true", "false", "iota", "nil", "append", "cap", "clear", "close", "complex", "copy", "delete", "imag", "len", "make", "max", "min", "new", "panic", "print", "println", "real", "recover", "any", "comparable", "main", "init", "self",
];

pub fn ident(name: &str) -> String {
    let n: String = name.chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if n == "self" {
        return "self".into();
    }
    if RESERVED.contains(&n.as_str()) {
        if n == "main" { "wj_main".to_string() } else { format!("{n}_") }
    } else if n.starts_with("wj") {
        format!("u_{n}")
    } else {
        n
    }
}

/// Exported field name (tuple-like `0` -> `F0`; `x` -> `X`) so reflection-based clone/debug work.
pub fn field(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_digit()) {
        return format!("F{name}");
    }
    let mut cs = name.chars();
    match cs.next() {
        Some(f) => format!("{}{}", f.to_uppercase(), cs.as_str()),
        None => String::new(),
    }
}

pub fn variant_type(en: &str, var: &str) -> String {
    format!("{}_{}", ident(en), var)
}

pub fn go_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => out.push_str(&format!("\\x{:02x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}
