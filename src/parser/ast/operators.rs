// Operators - Binary and unary operators for Windjammer

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,    // Logical AND (&&)
    Or,     // Logical OR (||)
    BitAnd, // Bitwise AND (&)
    BitOr,  // Bitwise OR (|)
    BitXor, // Bitwise XOR (^)
    Shl,    // Shift left (<<)
    Shr,    // Shift right (>>)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,
    Neg,
    Ref,    // & operator
    MutRef, // &mut operator
    Deref,  // * operator (dereference)
}

// Compound assignment operators
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompoundOp {
    Add,    // +=
    Sub,    // -=
    Mul,    // *=
    Div,    // /=
    Mod,    // %=
    BitAnd, // &=
    BitOr,  // |=
    BitXor, // ^=
    Shl,    // <<=
    Shr,    // >>=
}
