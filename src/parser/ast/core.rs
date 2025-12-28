// AST Core Types - Circular dependencies
//
// This file contains types with circular dependencies that must stay together:
// Expression ↔ Statement ↔ Pattern
//
// Independent types have been extracted to separate modules.

// Import types from extracted modules
use crate::parser::ast::literals::{Literal, MacroDelimiter};
use crate::parser::ast::operators::{BinaryOp, CompoundOp, UnaryOp};
use crate::parser::ast::ownership::OwnershipHint;
use crate::parser::ast::types::{AssociatedType, SourceLocation, Type, TypeParam};

// ============================================================================
// PARAMETERS (depends on Pattern - circular)
// ============================================================================

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
pub struct Decorator<'ast> {
    pub name: String,
    pub arguments: Vec<(String, &'ast Expression<'ast>)>, // Named arguments
}

// ============================================================================
// FUNCTIONS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionDecl<'ast> {
    pub name: String,
    pub is_pub: bool,                // Whether this function has pub visibility
    pub is_extern: bool,             // Whether this is an extern function (FFI)
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: <T: Display, U>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub decorators: Vec<Decorator<'ast>>,
    pub is_async: bool,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: Vec<&'ast Statement<'ast>>,        // Empty for extern functions
    pub parent_type: Option<String>, // The type name if this function is in an impl block
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
}

// ============================================================================
// STRUCTS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructField<'ast> {
    pub name: String,
    pub field_type: Type,
    pub decorators: Vec<Decorator<'ast>>,
    pub is_pub: bool,
    pub doc_comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructDecl<'ast> {
    pub name: String,
    pub is_pub: bool,                // Whether this struct has pub visibility
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: <T: Clone>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub fields: Vec<StructField<'ast>>,
    pub decorators: Vec<Decorator<'ast>>,
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
}

// ============================================================================
// ENUMS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumVariant {
    pub name: String,
    pub data: EnumVariantData,
    pub doc_comment: Option<String>,
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
pub enum Statement<'ast> {
    Let {
        pattern: Pattern<'ast>,
        mutable: bool,
        type_: Option<Type>,
        value: &'ast Expression<'ast>,
        /// Optional else block for let-else patterns (e.g., `let Some(x) = opt else { return }`)
        else_block: Option<Vec<&'ast Statement<'ast>>>,
        location: SourceLocation,
    },
    Const {
        name: String,
        type_: Type,
        value: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Assignment {
        target: &'ast Expression<'ast>,
        value: &'ast Expression<'ast>,
        compound_op: Option<CompoundOp>,
        location: SourceLocation,
    },
    Return {
        value: Option<&'ast Expression<'ast>>,
        location: SourceLocation,
    },
    Expression {
        expr: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    If {
        condition: &'ast Expression<'ast>,
        then_block: Vec<&'ast Statement<'ast>>,
        else_block: Option<Vec<&'ast Statement<'ast>>>,
        location: SourceLocation,
    },
    Match {
        value: &'ast Expression<'ast>,
        arms: Vec<MatchArm<'ast>>,
        location: SourceLocation,
    },
    For {
        pattern: Pattern<'ast>,
        iterable: &'ast Expression<'ast>,
        body: Vec<&'ast Statement<'ast>>,
        location: SourceLocation,
    },
    Loop {
        body: Vec<&'ast Statement<'ast>>,
        location: SourceLocation,
    },
    While {
        condition: &'ast Expression<'ast>,
        body: Vec<&'ast Statement<'ast>>,
        location: SourceLocation,
    },
    Thread {
        body: Vec<&'ast Statement<'ast>>,
        location: SourceLocation,
    },
    Async {
        body: Vec<&'ast Statement<'ast>>,
        location: SourceLocation,
    },
    Defer {
        statement: &'ast Statement<'ast>,
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
        is_pub: bool, // THE WINDJAMMER WAY: Track pub use for re-exports
        location: SourceLocation,
    },
}

impl<'ast> Statement<'ast> {
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
pub struct MatchArm<'ast> {
    pub pattern: Pattern<'ast>,
    pub guard: Option<&'ast Expression<'ast>>, // Optional guard: if condition
    pub body: &'ast Expression<'ast>,
}

// ============================================================================
// PATTERNS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EnumPatternBinding {
    None,                                 // No parentheses: None, Empty
    Wildcard,                             // Parentheses with wildcard: Some(_)
    Single(String),                       // Single binding: Some(x)
    Tuple(Vec<Pattern>),                  // Multiple bindings: Rgb(r, g, b)
    Struct(Vec<(String, Pattern)>, bool), // Struct pattern: Box { width: w, height: h }, bool=has_wildcard (..)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Pattern<'ast> {
    Wildcard,
    Identifier(String),
    EnumVariant(String, EnumPatternBinding), // Enum name, binding type
    Literal(Literal),
    Tuple(Vec<Pattern<'ast>>),         // Tuple pattern: (a, b, c)
    Or(Vec<Pattern<'ast>>),            // Or pattern: pattern1 | pattern2 | pattern3
    Reference(&'ast Pattern<'ast>),     // Reference pattern: &x
}

// ============================================================================
// EXPRESSIONS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Expression<'ast> {
    Literal {
        value: Literal,
        location: SourceLocation,
    },
    Identifier {
        name: String,
        location: SourceLocation,
    },
    Binary {
        left: &'ast Expression<'ast>,
        op: BinaryOp,
        right: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Unary {
        op: UnaryOp,
        operand: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Call {
        function: &'ast Expression<'ast>,
        arguments: Vec<(Option<String>, &'ast Expression<'ast>)>, // (label, expr)
        location: SourceLocation,
    },
    MethodCall {
        object: &'ast Expression<'ast>,
        method: String,
        type_args: Option<Vec<Type>>, // Turbofish: Vec::<int>::new()
        arguments: Vec<(Option<String>, &'ast Expression<'ast>)>, // (label, expr)
        location: SourceLocation,
    },
    FieldAccess {
        object: &'ast Expression<'ast>,
        field: String,
        location: SourceLocation,
    },
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression<'ast>)>,
        location: SourceLocation,
    },
    MapLiteral {
        pairs: Vec<(Expression<'ast>, Expression<'ast>)>, // {key: value, ...}
        location: SourceLocation,
    },
    Range {
        start: &'ast Expression<'ast>,
        end: &'ast Expression<'ast>,
        inclusive: bool,
        location: SourceLocation,
    },
    Closure {
        parameters: Vec<String>,
        body: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Cast {
        expr: &'ast Expression<'ast>,
        type_: Type,
        location: SourceLocation,
    },
    Index {
        object: &'ast Expression<'ast>,
        index: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Tuple {
        elements: Vec<&'ast Expression<'ast>>, // Tuple expression: (a, b, c)
        location: SourceLocation,
    },
    Array {
        elements: Vec<&'ast Expression<'ast>>, // Array expression: [a, b, c]
        location: SourceLocation,
    },
    MacroInvocation {
        name: String,
        args: Vec<&'ast Expression<'ast>>,
        delimiter: MacroDelimiter, // (), [], or {}
        location: SourceLocation,
    },
    TryOp {
        expr: &'ast Expression<'ast>, // The ? operator
        location: SourceLocation,
    },
    Await {
        expr: &'ast Expression<'ast>, // The .await
        location: SourceLocation,
    },
    ChannelSend {
        channel: &'ast Expression<'ast>,
        value: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    ChannelRecv {
        channel: &'ast Expression<'ast>,
        location: SourceLocation,
    },
    Block {
        statements: Vec<Statement<'ast>>,
        location: SourceLocation,
    },
}

impl<'ast> Expression<'ast> {
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
// TRAITS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitDecl<'ast> {
    pub name: String,
    pub generics: Vec<String>,    // Generic parameters like <T, U>
    pub supertraits: Vec<String>, // Supertrait bounds: trait Manager: Employee
    pub associated_types: Vec<AssociatedType>, // Associated type declarations: type Item;
    pub methods: Vec<TraitMethod<'ast>>,
    pub doc_comment: Option<String>, // Documentation comment (/// lines)
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraitMethod<'ast> {
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub is_async: bool,
    pub body: Option<Vec<Statement<'ast>>>, // None for trait definitions, Some for default impls
    pub doc_comment: Option<String>,  // Documentation comment (/// lines)
}

// ============================================================================
// IMPL BLOCKS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImplBlock<'ast> {
    pub type_name: String,
    pub type_params: Vec<TypeParam>, // Generic type parameters with optional bounds: impl<T: Display> Box<T>
    pub where_clause: Vec<(String, Vec<String>)>, // Where clause: [(type_param, [trait_bounds])]
    pub trait_name: Option<String>, // None for inherent impl, Some for trait impl (without type args)
    pub trait_type_args: Option<Vec<Type>>, // Type arguments for generic trait impl: From<int> -> Some([Type::Int])
    pub associated_types: Vec<AssociatedType>, // Associated type implementations: type Item = i32;
    pub functions: Vec<FunctionDecl<'ast>>,
    pub decorators: Vec<Decorator<'ast>>,
}

// ============================================================================
// TOP-LEVEL ITEMS
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Item<'ast> {
    Function {
        decl: FunctionDecl<'ast>,
        location: SourceLocation,
    },
    Struct {
        decl: StructDecl<'ast>,
        location: SourceLocation,
    },
    Enum {
        decl: EnumDecl,
        location: SourceLocation,
    },
    Trait {
        decl: TraitDecl<'ast>,
        location: SourceLocation,
    },
    Impl {
        block: ImplBlock<'ast>,
        location: SourceLocation,
    },
    Const {
        name: String,
        type_: Type,
        value: Expression<'ast>,
        location: SourceLocation,
    },
    Static {
        name: String,
        mutable: bool,
        type_: Type,
        value: Expression<'ast>,
        location: SourceLocation,
    },
    Use {
        path: Vec<String>,
        alias: Option<String>,
        is_pub: bool, // THE WINDJAMMER WAY: Track pub use for re-exports
        location: SourceLocation,
    }, // use std::fs as fs -> path=["std", "fs"], alias=Some("fs")
    Mod {
        name: String,
        items: Vec<Item<'ast>>,
        is_public: bool,
        location: SourceLocation,
    }, // mod ffi { ... }
    BoundAlias {
        name: String,
        traits: Vec<String>,
        location: SourceLocation,
    }, // bound Printable = Display + Debug
}

impl<'ast> Item<'ast> {
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
pub struct Program<'ast> {
    pub items: Vec<Item<'ast>>,
}
