// AST Types - Windjammer Abstract Syntax Tree Definitions
//
// This module contains all AST (Abstract Syntax Tree) type definitions for Windjammer.
// These types represent the parsed structure of Windjammer source code.

// ============================================================================
// TYPE SYSTEM
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Int,
    Int32,
    Uint,
    Float,
    Bool,
    String,
    Custom(String),
    Generic(String),                  // Type parameter: T, U, V
    Parameterized(String, Vec<Type>), // Generic type: Vec<T>, HashMap<K, V>
    Associated(String, String),       // Associated type: Self::Item, T::Output (base, assoc_name)
    TraitObject(String),              // Trait object: dyn Trait
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Vec(Box<Type>),          // Dynamic array: Vec<T>
    Array(Box<Type>, usize), // Fixed-size array: [T; N]
    Reference(Box<Type>),
    MutableReference(Box<Type>),
    Tuple(Vec<Type>), // Tuple type: (T1, T2, T3)
    Infer,            // Type inference placeholder: _
    FunctionPointer {
        params: Vec<Type>,
        return_type: Option<Box<Type>>,
    }, // Function pointer: fn(int, string) -> bool
}

// Type parameter with optional trait bounds
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeParam {
    pub name: String,
    pub bounds: Vec<String>, // Trait bounds: ["Display", "Clone", "Send"]
}

// Associated type declaration in traits or implementation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AssociatedType {
    pub name: String,                // e.g., "Item", "Output"
    pub concrete_type: Option<Type>, // None in trait declaration, Some(Type) in impl
}

// ============================================================================
// PARAMETERS AND OWNERSHIP
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OwnershipHint {
    Owned,
    Ref,
    Mut,
    Inferred, // Let the analyzer decide
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Parameter {
    pub name: String,             // For simple parameters and backward compatibility
    pub pattern: Option<Pattern>, // For pattern matching parameters
    pub type_: Type,
    pub ownership: OwnershipHint,
}

// ============================================================================
// DECORATORS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Decorator {
    pub name: String,
    pub arguments: Vec<(String, Expression)>, // Named arguments
}

// ============================================================================
// FUNCTIONS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: <T: Display, U>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub decorators: Vec<Decorator>,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,
}

// ============================================================================
// STRUCTS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField {
    pub name: String,
    pub field_type: Type,
    pub decorators: Vec<Decorator>,
    pub is_pub: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: <T: Clone>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub fields: Vec<StructField>,
    pub decorators: Vec<Decorator>,
}

// ============================================================================
// ENUMS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub data: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDecl {
    pub name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters: enum Option<T>, enum Result<T, E>
    pub variants: Vec<EnumVariant>,
}

// ============================================================================
// STATEMENTS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Statement {
    Let {
        pattern: Pattern,
        mutable: bool,
        type_: Option<Type>,
        value: Expression,
    },
    Const {
        name: String,
        type_: Type,
        value: Expression,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: Expression,
    },
    Assignment {
        target: Expression,
        value: Expression,
    },
    Return(Option<Expression>),
    Expression(Expression),
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    Match {
        value: Expression,
        arms: Vec<MatchArm>,
    },
    For {
        pattern: Pattern,
        iterable: Expression,
        body: Vec<Statement>,
    },
    Loop {
        body: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Thread {
        body: Vec<Statement>,
    },
    Async {
        body: Vec<Statement>,
    },
    Defer(Box<Statement>),
    Break,
    Continue,
    Use {
        path: Vec<String>,
        alias: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expression>, // Optional guard: if condition
    pub body: Expression,
}

// ============================================================================
// PATTERNS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumPatternBinding {
    None,          // No parentheses: None, Empty
    Wildcard,      // Parentheses with wildcard: Some(_)
    Named(String), // Parentheses with binding: Some(x)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern {
    Wildcard,
    Identifier(String),
    EnumVariant(String, EnumPatternBinding), // Enum name, binding type
    Literal(Literal),
    Tuple(Vec<Pattern>),     // Tuple pattern: (a, b, c)
    Or(Vec<Pattern>),        // Or pattern: pattern1 | pattern2 | pattern3
    Reference(Box<Pattern>), // Reference pattern: &x
}

// ============================================================================
// EXPRESSIONS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression {
    Literal(Literal),
    Identifier(String),
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
    },
    Call {
        function: Box<Expression>,
        arguments: Vec<(Option<String>, Expression)>, // (label, expr)
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        type_args: Option<Vec<Type>>, // Turbofish: Vec::<int>::new()
        arguments: Vec<(Option<String>, Expression)>, // (label, expr)
    },
    FieldAccess {
        object: Box<Expression>,
        field: String,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    MapLiteral(Vec<(Expression, Expression)>), // {key: value, ...}
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
    },
    Closure {
        parameters: Vec<String>,
        body: Box<Expression>,
    },
    Cast {
        expr: Box<Expression>,
        type_: Type,
    },
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    Tuple(Vec<Expression>), // Tuple expression: (a, b, c)
    Array(Vec<Expression>), // Array expression: [a, b, c]
    MacroInvocation {
        name: String,
        args: Vec<Expression>,
        delimiter: MacroDelimiter, // (), [], or {}
    },
    TryOp(Box<Expression>), // The ? operator
    Await(Box<Expression>), // The .await
    ChannelSend {
        channel: Box<Expression>,
        value: Box<Expression>,
    },
    ChannelRecv(Box<Expression>),
    Block(Vec<Statement>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MacroDelimiter {
    Parens,   // println!()
    Brackets, // vec![]
    Braces,   // macro_name!{}
}

// ============================================================================
// LITERALS
// ============================================================================

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

// Manual Hash implementation for Literal (needed because f64 doesn't implement Hash)
impl std::hash::Hash for Literal {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Literal::Int(i) => {
                0u8.hash(state);
                i.hash(state);
            }
            Literal::Float(f) => {
                1u8.hash(state);
                // Hash the bit representation of the float
                f.to_bits().hash(state);
            }
            Literal::String(s) => {
                2u8.hash(state);
                s.hash(state);
            }
            Literal::Char(c) => {
                3u8.hash(state);
                c.hash(state);
            }
            Literal::Bool(b) => {
                4u8.hash(state);
                b.hash(state);
            }
        }
    }
}

// ============================================================================
// OPERATORS
// ============================================================================

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
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Not,
    Neg,
    Ref,    // & operator
    MutRef, // &mut operator
    Deref,  // * operator (dereference)
}

// ============================================================================
// TRAITS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitDecl {
    pub name: String,
    pub generics: Vec<String>,    // Generic parameters like <T, U>
    pub supertraits: Vec<String>, // Supertrait bounds: trait Manager: Employee
    pub associated_types: Vec<AssociatedType>, // Associated type declarations: type Item;
    pub methods: Vec<TraitMethod>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitMethod {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub is_async: bool,
    pub body: Option<Vec<Statement>>, // None for trait definitions, Some for default impls
}

// ============================================================================
// IMPL BLOCKS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplBlock {
    pub type_name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: impl<T: Display> Box<T>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub trait_name: Option<String>, // None for inherent impl, Some for trait impl (without type args)
    pub trait_type_args: Option<Vec<Type>>, // Type arguments for generic trait impl: From<int> -> Some([Type::Int])
    pub associated_types: Vec<AssociatedType>, // Associated type implementations: type Item = i32;
    pub functions: Vec<FunctionDecl>,
    pub decorators: Vec<Decorator>,
}

// ============================================================================
// TOP-LEVEL ITEMS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item {
    Function(FunctionDecl),
    Struct(StructDecl),
    Enum(EnumDecl),
    Trait(TraitDecl),
    Impl(ImplBlock),
    Const {
        name: String,
        type_: Type,
        value: Expression,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: Expression,
    },
    Use {
        path: Vec<String>,
        alias: Option<String>,
    }, // use std::fs as fs -> path=["std", "fs"], alias=Some("fs")
    BoundAlias {
        name: String,
        traits: Vec<String>,
    }, // bound Printable = Display + Debug
}

// ============================================================================
// PROGRAM
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Program {
    pub items: Vec<Item>,
}
