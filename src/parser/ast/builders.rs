// AST Builders - Ergonomic construction of AST nodes
//
// Provides builder functions to make test code dramatically more concise:
//
// Before: Type::Reference(Box::new(Type::Vec(Box::new(Type::Int))))
// After:  type_ref(type_vec(Type::Int))
//
// Expected impact: 60-80% reduction in test code lines

use super::core::{Expression, Parameter, Pattern, Statement};
use super::literals::{Literal, MacroDelimiter};
use super::operators::{BinaryOp, CompoundOp, UnaryOp};
use super::ownership::OwnershipHint;
use super::types::Type;

// ============================================================================
// TYPE BUILDERS
// ============================================================================

/// Build primitive type: int
pub fn type_int() -> Type {
    Type::Int
}

/// Build primitive type: float
pub fn type_float() -> Type {
    Type::Float
}

/// Build primitive type: bool
pub fn type_bool() -> Type {
    Type::Bool
}

/// Build primitive type: string
pub fn type_string() -> Type {
    Type::String
}

/// Build custom type: MyType
pub fn type_custom(name: impl Into<String>) -> Type {
    Type::Custom(name.into())
}

/// Build reference type: &T
pub fn type_ref(inner: Type) -> Type {
    Type::Reference(Box::new(inner))
}

/// Build mutable reference type: &mut T
pub fn type_mut_ref(inner: Type) -> Type {
    Type::MutableReference(Box::new(inner))
}

/// Build Vec type: Vec<T>
pub fn type_vec(inner: Type) -> Type {
    Type::Vec(Box::new(inner))
}

/// Build Option type: Option<T>
pub fn type_option(inner: Type) -> Type {
    Type::Option(Box::new(inner))
}

/// Build Result type: Result<T, E>
pub fn type_result(ok: Type, err: Type) -> Type {
    Type::Result(Box::new(ok), Box::new(err))
}

/// Build parameterized type: HashMap<K, V>
pub fn type_parameterized(name: impl Into<String>, args: Vec<Type>) -> Type {
    Type::Parameterized(name.into(), args)
}

/// Build tuple type: (T1, T2, ...)
pub fn type_tuple(elements: Vec<Type>) -> Type {
    Type::Tuple(elements)
}

/// Build array type: [T; N]
pub fn type_array(inner: Type, size: usize) -> Type {
    Type::Array(Box::new(inner), size)
}

/// Build generic type parameter: T
pub fn type_generic(name: impl Into<String>) -> Type {
    Type::Generic(name.into())
}

/// Build associated type: Self::Item
pub fn type_associated(base: impl Into<String>, assoc: impl Into<String>) -> Type {
    Type::Associated(base.into(), assoc.into())
}

/// Build trait object: dyn Trait
pub fn type_trait_object(name: impl Into<String>) -> Type {
    Type::TraitObject(name.into())
}

/// Build type inference placeholder: _
pub fn type_infer() -> Type {
    Type::Infer
}

/// Build int32 type
pub fn type_int32() -> Type {
    Type::Int32
}

/// Build uint type
pub fn type_uint() -> Type {
    Type::Uint
}

// ============================================================================
// PARAMETER BUILDERS
// ============================================================================

/// Build simple parameter with inferred ownership
pub fn param<'ast>(name: impl Into<String>, type_: Type) -> Parameter<'ast> {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Inferred,
        is_mutable: false,
    }
}

/// Build reference parameter
pub fn param_ref<'ast>(name: impl Into<String>, type_: Type) -> Parameter<'ast> {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Ref,
        is_mutable: false,
    }
}

/// Build mutable reference parameter
pub fn param_mut<'ast>(name: impl Into<String>, type_: Type) -> Parameter<'ast> {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Mut,
        is_mutable: true,
    }
}

/// Build owned parameter
pub fn param_owned<'ast>(name: impl Into<String>, type_: Type) -> Parameter<'ast> {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Owned,
        is_mutable: false,
    }
}

// ============================================================================
// EXPRESSION BUILDERS
// ============================================================================

/// Build integer literal expression
pub fn expr_int<'ast>(value: i64) -> Expression<'ast> {
    Expression::Literal {
        value: Literal::Int(value),
        location: None,
    }
}

/// Build float literal expression
pub fn expr_float<'ast>(value: f64) -> Expression<'ast> {
    Expression::Literal {
        value: Literal::Float(value),
        location: None,
    }
}

/// Build string literal expression
pub fn expr_string<'ast>(value: impl Into<String>) -> Expression<'ast> {
    Expression::Literal {
        value: Literal::String(value.into()),
        location: None,
    }
}

/// Build boolean literal expression
pub fn expr_bool<'ast>(value: bool) -> Expression<'ast> {
    Expression::Literal {
        value: Literal::Bool(value),
        location: None,
    }
}

/// Build character literal expression
pub fn expr_char<'ast>(value: char) -> Expression<'ast> {
    Expression::Literal {
        value: Literal::Char(value),
        location: None,
    }
}

/// Build variable/identifier expression
pub fn expr_var<'ast>(name: impl Into<String>) -> Expression<'ast> {
    Expression::Identifier {
        name: name.into(),
        location: None,
    }
}

/// Build binary operation expression
pub fn expr_binary<'ast>(op: BinaryOp, left: &'ast Expression<'ast>, right: &'ast Expression<'ast>) -> Expression<'ast> {
    Expression::Binary {
        left,
        op,
        right,
        location: None,
    }
}

/// Build addition expression (convenience)
pub fn expr_add<'ast>(left: &'ast Expression<'ast>, right: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_binary(BinaryOp::Add, left, right)
}

/// Build subtraction expression (convenience)
pub fn expr_sub<'ast>(left: &'ast Expression<'ast>, right: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_binary(BinaryOp::Sub, left, right)
}

/// Build multiplication expression (convenience)
pub fn expr_mul<'ast>(left: &'ast Expression<'ast>, right: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_binary(BinaryOp::Mul, left, right)
}

/// Build division expression (convenience)
pub fn expr_div<'ast>(left: &'ast Expression<'ast>, right: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_binary(BinaryOp::Div, left, right)
}

/// Build equality expression (convenience)
pub fn expr_eq<'ast>(left: &'ast Expression<'ast>, right: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_binary(BinaryOp::Eq, left, right)
}

/// Build unary operation expression
pub fn expr_unary<'ast>(op: UnaryOp, operand: &'ast Expression<'ast>) -> Expression<'ast> {
    Expression::Unary {
        op,
        operand,
        location: None,
    }
}

/// Build negation expression (convenience)
pub fn expr_neg<'ast>(operand: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_unary(UnaryOp::Neg, operand)
}

/// Build logical NOT expression (convenience)
pub fn expr_not<'ast>(operand: &'ast Expression<'ast>) -> Expression<'ast> {
    expr_unary(UnaryOp::Not, operand)
}

/// Build function call expression
/// Note: Caller must provide function expression allocated in arena
pub fn expr_call<'ast>(function: &'ast Expression<'ast>, arguments: Vec<(Option<String>, &'ast Expression<'ast>)>) -> Expression<'ast> {
    Expression::Call {
        function,
        arguments,
        location: None,
    }
}

/// Build method call expression
pub fn expr_method<'ast>(
    object: &'ast Expression<'ast>,
    method: impl Into<String>,
    arguments: Vec<(Option<String>, &'ast Expression<'ast>)>,
) -> Expression<'ast> {
    Expression::MethodCall {
        object,
        method: method.into(),
        type_args: None,
        arguments,
        location: None,
    }
}

/// Build macro invocation expression
pub fn expr_macro<'ast>(name: impl Into<String>, args: Vec<&'ast Expression<'ast>>) -> Expression<'ast> {
    Expression::MacroInvocation {
        name: name.into(),
        args,
        delimiter: MacroDelimiter::Parens,
        location: None,
    }
}

/// Build field access expression
pub fn expr_field<'ast>(object: &'ast Expression<'ast>, field: impl Into<String>) -> Expression<'ast> {
    Expression::FieldAccess {
        object,
        field: field.into(),
        location: None,
    }
}

/// Build array index expression
pub fn expr_index<'ast>(array: &'ast Expression<'ast>, index: &'ast Expression<'ast>) -> Expression<'ast> {
    Expression::Index {
        object: array,
        index,
        location: None,
    }
}

/// Build array literal expression
pub fn expr_array<'ast>(elements: Vec<&'ast Expression<'ast>>) -> Expression<'ast> {
    Expression::Array {
        elements,
        location: None,
    }
}

/// Build tuple expression
pub fn expr_tuple<'ast>(elements: Vec<&'ast Expression<'ast>>) -> Expression<'ast> {
    Expression::Tuple {
        elements,
        location: None,
    }
}

/// Build block expression
pub fn expr_block<'ast>(statements: Vec<&'ast Statement<'ast>>) -> Expression<'ast> {
    Expression::Block {
        statements,
        location: None,
    }
}

// ============================================================================
// STATEMENT BUILDERS
// ============================================================================

/// Build let statement (immutable)
pub fn stmt_let<'ast>(name: impl Into<String>, type_: Option<Type>, value: &'ast Expression<'ast>) -> Statement<'ast> {
    Statement::Let {
        pattern: Pattern::Identifier(name.into()),
        mutable: false,
        type_,
        value,
        else_block: None,
        location: None,
    }
}

/// Build let mut statement (mutable)
pub fn stmt_let_mut<'ast>(name: impl Into<String>, type_: Option<Type>, value: &'ast Expression<'ast>) -> Statement<'ast> {
    Statement::Let {
        pattern: Pattern::Identifier(name.into()),
        mutable: true,
        type_,
        value,
        else_block: None,
        location: None,
    }
}

/// Build assignment statement
pub fn stmt_assign<'ast>(target: &'ast Expression<'ast>, value: &'ast Expression<'ast>) -> Statement<'ast> {
    Statement::Assignment {
        target,
        value,
        compound_op: None,
        location: None,
    }
}

/// Build compound assignment statement (+=, -=, etc.)
pub fn stmt_compound_assign<'ast>(op: CompoundOp, target: &'ast Expression<'ast>, value: &'ast Expression<'ast>) -> Statement<'ast> {
    Statement::Assignment {
        target,
        value,
        compound_op: Some(op),
        location: None,
    }
}

/// Build return statement
pub fn stmt_return<'ast>(value: Option<&'ast Expression<'ast>>) -> Statement<'ast> {
    Statement::Return {
        value,
        location: None,
    }
}

/// Build expression statement
pub fn stmt_expr<'ast>(expr: &'ast Expression<'ast>) -> Statement<'ast> {
    Statement::Expression {
        expr,
        location: None,
    }
}

/// Build if statement (no else)
pub fn stmt_if<'ast>(
    condition: &'ast Expression<'ast>,
    then_block: Vec<&'ast Statement<'ast>>,
    else_block: Option<Vec<&'ast Statement<'ast>>>,
) -> Statement<'ast> {
    Statement::If {
        condition,
        then_block,
        else_block,
        location: None,
    }
}

/// Build if-else statement (convenience)
pub fn stmt_if_else<'ast>(
    condition: &'ast Expression<'ast>,
    then_block: Vec<&'ast Statement<'ast>>,
    else_block: Vec<&'ast Statement<'ast>>,
) -> Statement<'ast> {
    Statement::If {
        condition,
        then_block,
        else_block: Some(else_block),
        location: None,
    }
}

/// Build while loop statement
pub fn stmt_while<'ast>(condition: &'ast Expression<'ast>, body: Vec<&'ast Statement<'ast>>) -> Statement<'ast> {
    Statement::While {
        condition,
        body,
        location: None,
    }
}

/// Build infinite loop statement
pub fn stmt_loop<'ast>(body: Vec<&'ast Statement<'ast>>) -> Statement<'ast> {
    Statement::Loop {
        body,
        location: None,
    }
}

/// Build break statement
pub fn stmt_break<'ast>() -> Statement<'ast> {
    Statement::Break { location: None }
}

/// Build continue statement
pub fn stmt_continue<'ast>() -> Statement<'ast> {
    Statement::Continue { location: None }
}

/// Build for loop statement
pub fn stmt_for<'ast>(pattern: Pattern<'ast>, iterable: &'ast Expression<'ast>, body: Vec<&'ast Statement<'ast>>) -> Statement<'ast> {
    Statement::For {
        pattern,
        iterable,
        body,
        location: None,
    }
}

/// Build match statement
pub fn stmt_match<'ast>(value: &'ast Expression<'ast>, arms: Vec<super::core::MatchArm<'ast>>) -> Statement<'ast> {
    Statement::Match {
        value,
        arms,
        location: None,
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_primitives() {
        assert_eq!(type_int(), Type::Int);
        assert_eq!(type_float(), Type::Float);
        assert_eq!(type_bool(), Type::Bool);
        assert_eq!(type_string(), Type::String);
    }

    #[test]
    fn test_type_custom() {
        assert_eq!(type_custom("MyType"), Type::Custom("MyType".to_string()));
    }

    #[test]
    fn test_type_reference() {
        assert_eq!(type_ref(Type::Int), Type::Reference(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_mut_reference() {
        assert_eq!(
            type_mut_ref(Type::Int),
            Type::MutableReference(Box::new(Type::Int))
        );
    }

    #[test]
    fn test_type_vec() {
        assert_eq!(type_vec(Type::Int), Type::Vec(Box::new(Type::Int)));
    }

    #[test]
    fn test_type_option() {
        assert_eq!(
            type_option(Type::String),
            Type::Option(Box::new(Type::String))
        );
    }

    #[test]
    fn test_type_result() {
        assert_eq!(
            type_result(Type::Int, Type::String),
            Type::Result(Box::new(Type::Int), Box::new(Type::String))
        );
    }

    #[test]
    fn test_type_parameterized() {
        assert_eq!(
            type_parameterized("Vec", vec![Type::Int]),
            Type::Parameterized("Vec".to_string(), vec![Type::Int])
        );
    }

    #[test]
    fn test_type_tuple() {
        assert_eq!(
            type_tuple(vec![Type::Int, Type::String]),
            Type::Tuple(vec![Type::Int, Type::String])
        );
    }

    #[test]
    fn test_type_chained() {
        // Complex: Option<&mut Vec<String>>
        let complex = type_option(type_mut_ref(type_vec(Type::String)));

        assert_eq!(
            complex,
            Type::Option(Box::new(Type::MutableReference(Box::new(Type::Vec(
                Box::new(Type::String)
            )))))
        );
    }

    #[test]
    fn test_param_simple() {
        let p = param("x", Type::Int);

        assert_eq!(p.name, "x");
        assert_eq!(p.type_, Type::Int);
        assert_eq!(p.ownership, OwnershipHint::Inferred);
        assert!(!p.is_mutable);
    }

    #[test]
    fn test_param_ref() {
        let p = param_ref("x", Type::Int);

        assert_eq!(p.ownership, OwnershipHint::Ref);
        assert!(!p.is_mutable);
    }

    #[test]
    fn test_param_mut() {
        let p = param_mut("x", Type::Int);

        assert_eq!(p.ownership, OwnershipHint::Mut);
        assert!(p.is_mutable);
    }

    #[test]
    fn test_param_owned() {
        let p = param_owned("data", Type::String);

        assert_eq!(p.ownership, OwnershipHint::Owned);
        assert!(!p.is_mutable);
    }

    #[test]
    fn test_ergonomic_example() {
        // Before builders (7 lines):
        // Parameter {
        //     name: "data".to_string(),
        //     pattern: None,
        //     type_: Type::Reference(Box::new(Type::Vec(Box::new(Type::Int)))),
        //     ownership: OwnershipHint::Ref,
        //     is_mutable: false,
        // }

        // After builders (1 line):
        let param = param_ref("data", type_vec(Type::Int));

        assert_eq!(param.name, "data");
        assert_eq!(param.type_, Type::Vec(Box::new(Type::Int)));
        assert_eq!(param.ownership, OwnershipHint::Ref);
    }
}
