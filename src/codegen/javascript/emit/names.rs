//! Identifier mangling for JavaScript.

const RESERVED: &[&str] = &[
    "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum", "export", "extends", "false", "finally", "for", "function", "if", "import", "in", "instanceof", "let", "new", "null", "return", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with", "yield", "await", "async", "static", "implements", "interface", "package", "private", "protected", "public", "arguments", "eval", "undefined", "NaN", "Infinity", "Object", "Array", "Map", "Set", "String", "Number", "BigInt", "Math", "JSON", "Symbol", "Error", "Promise", "process", "console", "require", "module", "exports", "globalThis", "of",
];

/// Local variable / function name.
pub fn ident(name: &str) -> String {
    let n: String = name.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '$' { c } else { '_' }).collect();
    if RESERVED.contains(&n.as_str()) { format!("{n}_") } else { n }
}

/// Property name for a field (`0` -> `_0`).
pub fn prop(name: &str) -> String {
    if name.chars().all(|c| c.is_ascii_digit()) {
        format!("_{name}")
    } else if name == "constructor" || name == "__proto__" || name == "prototype" {
        format!("{name}_")
    } else {
        name.to_string()
    }
}

/// Class name of an enum variant.
pub fn variant_class(en: &str, var: &str) -> String {
    format!("{}${}", ident(en), var)
}

pub fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}
