// AST Types - Windjammer Abstract Syntax Tree Definitions
//
// This module contains all AST (Abstract Syntax Tree) type definitions for Windjammer.
// These types represent the parsed structure of Windjammer source code.

use crate::source_map::Location;

// ============================================================================
// SOURCE LOCATION
// ============================================================================

/// Optional source location for error reporting
/// None means location information is not available
pub type SourceLocation = Option<Location>;

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
    pub is_mutable: bool, // Whether parameter is declared with 'mut' keyword
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
    pub is_pub: bool,                // Whether this function has pub visibility
    pub is_extern: bool,             // Whether this is an extern function (FFI)
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: <T: Display, U>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub decorators: Vec<Decorator>,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<Statement>,        // Empty for extern functions
    pub parent_type: Option<String>, // The type name if this function is in an impl block
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
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
    pub is_pub: bool,                // Whether this struct has pub visibility
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: <T: Clone>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub fields: Vec<StructField>,
    pub decorators: Vec<Decorator>,
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
}

// ============================================================================
// ENUMS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub data: EnumVariantData,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumVariantData {
    Unit,                        // Variant
    Tuple(Vec<Type>),            // Variant(T1, T2)
    Struct(Vec<(String, Type)>), // Variant { field1: T1, field2: T2 }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDecl {
    pub name: String,
    pub is_pub: bool,                // Whether this enum has pub visibility
    pub type_params: Vec<TypeParam>, // Generic type parameters: enum Option<T>, enum Result<T, E>
    pub variants: Vec<EnumVariant>,
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
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
        /// Optional else block for let-else patterns (e.g., `let Some(x) = opt else { return }`)
        else_block: Option<Vec<Statement>>,
        location: SourceLocation,
    },
    Const {
        name: String,
        type_: Type,
        value: Expression,
        location: SourceLocation,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: Expression,
        location: SourceLocation,
    },
    Assignment {
        target: Expression,
        value: Expression,
        location: SourceLocation,
    },
    Return {
        value: Option<Expression>,
        location: SourceLocation,
    },
    Expression {
        expr: Expression,
        location: SourceLocation,
    },
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
        location: SourceLocation,
    },
    Match {
        value: Expression,
        arms: Vec<MatchArm>,
        location: SourceLocation,
    },
    For {
        pattern: Pattern,
        iterable: Expression,
        body: Vec<Statement>,
        location: SourceLocation,
    },
    Loop {
        body: Vec<Statement>,
        location: SourceLocation,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
        location: SourceLocation,
    },
    Thread {
        body: Vec<Statement>,
        location: SourceLocation,
    },
    Async {
        body: Vec<Statement>,
        location: SourceLocation,
    },
    Defer {
        statement: Box<Statement>,
        location: SourceLocation,
    },
    Break {
        location: SourceLocation,
    },
    Continue {
        location: SourceLocation,
    },
    Use {
        path: Vec<String>,
        alias: Option<String>,
        location: SourceLocation,
    },
}

impl Statement {
    /// Get the source location of this statement (if available)
    pub fn location(&self) -> SourceLocation {
        match self {
            Statement::Let { location, .. } => location.clone(),
            Statement::Const { location, .. } => location.clone(),
            Statement::Static { location, .. } => location.clone(),
            Statement::Assignment { location, .. } => location.clone(),
            Statement::Return { location, .. } => location.clone(),
            Statement::Expression { location, .. } => location.clone(),
            Statement::If { location, .. } => location.clone(),
            Statement::Match { location, .. } => location.clone(),
            Statement::For { location, .. } => location.clone(),
            Statement::Loop { location, .. } => location.clone(),
            Statement::While { location, .. } => location.clone(),
            Statement::Thread { location, .. } => location.clone(),
            Statement::Async { location, .. } => location.clone(),
            Statement::Defer { location, .. } => location.clone(),
            Statement::Break { location, .. } => location.clone(),
            Statement::Continue { location, .. } => location.clone(),
            Statement::Use { location, .. } => location.clone(),
        }
    }
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
    None,                           // No parentheses: None, Empty
    Wildcard,                       // Parentheses with wildcard: Some(_)
    Single(String),                 // Single binding: Some(x)
    Tuple(Vec<Pattern>),            // Multiple bindings: Rgb(r, g, b)
    Struct(Vec<(String, Pattern)>), // Struct pattern: Box { width: w, height: h }
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
    Literal {
        value: Literal,
        location: SourceLocation,
    },
    Identifier {
        name: String,
        location: SourceLocation,
    },
    Binary {
        left: Box<Expression>,
        op: BinaryOp,
        right: Box<Expression>,
        location: SourceLocation,
    },
    Unary {
        op: UnaryOp,
        operand: Box<Expression>,
        location: SourceLocation,
    },
    Call {
        function: Box<Expression>,
        arguments: Vec<(Option<String>, Expression)>, // (label, expr)
        location: SourceLocation,
    },
    MethodCall {
        object: Box<Expression>,
        method: String,
        type_args: Option<Vec<Type>>, // Turbofish: Vec::<int>::new()
        arguments: Vec<(Option<String>, Expression)>, // (label, expr)
        location: SourceLocation,
    },
    FieldAccess {
        object: Box<Expression>,
        field: String,
        location: SourceLocation,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
        location: SourceLocation,
    },
    MapLiteral {
        pairs: Vec<(Expression, Expression)>, // {key: value, ...}
        location: SourceLocation,
    },
    Range {
        start: Box<Expression>,
        end: Box<Expression>,
        inclusive: bool,
        location: SourceLocation,
    },
    Closure {
        parameters: Vec<String>,
        body: Box<Expression>,
        is_move: bool, // move keyword for ownership-transferring closures
        location: SourceLocation,
    },
    Cast {
        expr: Box<Expression>,
        type_: Type,
        location: SourceLocation,
    },
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
        location: SourceLocation,
    },
    Tuple {
        elements: Vec<Expression>, // Tuple expression: (a, b, c)
        location: SourceLocation,
    },
    Array {
        elements: Vec<Expression>, // Array expression: [a, b, c]
        location: SourceLocation,
    },
    MacroInvocation {
        name: String,
        args: Vec<Expression>,
        delimiter: MacroDelimiter, // (), [], or {}
        location: SourceLocation,
    },
    TryOp {
        expr: Box<Expression>, // The ? operator
        location: SourceLocation,
    },
    Await {
        expr: Box<Expression>, // The .await
        location: SourceLocation,
    },
    ChannelSend {
        channel: Box<Expression>,
        value: Box<Expression>,
        location: SourceLocation,
    },
    ChannelRecv {
        channel: Box<Expression>,
        location: SourceLocation,
    },
    Block {
        statements: Vec<Statement>,
        location: SourceLocation,
    },
}

impl Expression {
    /// Get the source location of this expression (if available)
    pub fn location(&self) -> SourceLocation {
        match self {
            Expression::Literal { location, .. } => location.clone(),
            Expression::Identifier { location, .. } => location.clone(),
            Expression::Binary { location, .. } => location.clone(),
            Expression::Unary { location, .. } => location.clone(),
            Expression::Call { location, .. } => location.clone(),
            Expression::MethodCall { location, .. } => location.clone(),
            Expression::FieldAccess { location, .. } => location.clone(),
            Expression::StructLiteral { location, .. } => location.clone(),
            Expression::MapLiteral { location, .. } => location.clone(),
            Expression::Range { location, .. } => location.clone(),
            Expression::Closure { location, .. } => location.clone(),
            Expression::Cast { location, .. } => location.clone(),
            Expression::Index { location, .. } => location.clone(),
            Expression::Tuple { location, .. } => location.clone(),
            Expression::Array { location, .. } => location.clone(),
            Expression::MacroInvocation { location, .. } => location.clone(),
            Expression::TryOp { location, .. } => location.clone(),
            Expression::Await { location, .. } => location.clone(),
            Expression::ChannelSend { location, .. } => location.clone(),
            Expression::ChannelRecv { location, .. } => location.clone(),
            Expression::Block { location, .. } => location.clone(),
        }
    }
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
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitMethod {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub is_async: bool,
    pub body: Option<Vec<Statement>>, // None for trait definitions, Some for default impls
    pub doc_comment: Option<String>,  // Documentation comment (/// lines)
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
    Function {
        decl: FunctionDecl,
        location: SourceLocation,
    },
    Struct {
        decl: StructDecl,
        location: SourceLocation,
    },
    Enum {
        decl: EnumDecl,
        location: SourceLocation,
    },
    Trait {
        decl: TraitDecl,
        location: SourceLocation,
    },
    Impl {
        block: ImplBlock,
        location: SourceLocation,
    },
    Const {
        name: String,
        type_: Type,
        value: Expression,
        location: SourceLocation,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: Expression,
        location: SourceLocation,
    },
    Use {
        path: Vec<String>,
        alias: Option<String>,
        location: SourceLocation,
    }, // use std::fs as fs -> path=["std", "fs"], alias=Some("fs")
    Mod {
        name: String,
        items: Vec<Item>,
        is_public: bool,
        location: SourceLocation,
    }, // mod ffi { ... }
    BoundAlias {
        name: String,
        traits: Vec<String>,
        location: SourceLocation,
    }, // bound Printable = Display + Debug
}

impl Item {
    /// Get the source location of this item (if available)
    pub fn location(&self) -> SourceLocation {
        match self {
            Item::Function { location, .. } => location.clone(),
            Item::Struct { location, .. } => location.clone(),
            Item::Enum { location, .. } => location.clone(),
            Item::Trait { location, .. } => location.clone(),
            Item::Impl { location, .. } => location.clone(),
            Item::Const { location, .. } => location.clone(),
            Item::Static { location, .. } => location.clone(),
            Item::Use { location, .. } => location.clone(),
            Item::Mod { location, .. } => location.clone(),
            Item::BoundAlias { location, .. } => location.clone(),
        }
    }
}

// ============================================================================
// PROGRAM
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Program {
    pub items: Vec<Item>,
}
