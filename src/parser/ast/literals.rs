// Literals - Literal value types for Windjammer

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
}

// Manual Eq implementation (treats NaN == NaN for hashing purposes)
impl Eq for Literal {}

// MacroDelimiter for macro invocations
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroDelimiter {
    Parens,   // println!()
    Brackets, // vec![]
    Braces,   // macro_name!{}
}
