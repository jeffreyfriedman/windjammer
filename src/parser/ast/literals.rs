// Literals - Literal value types for Windjammer

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    IntSuffixed(i64, std::string::String), // 0u32, 100_000usize
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

// Manual Eq implementation (treats NaN == NaN for hashing purposes)
impl Eq for Literal {}

// MacroDelimiter for macro invocations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MacroDelimiter {
    Parens,   // println!()
    Brackets, // vec![]
    Braces,   // macro_name!{}
}
