// AST Builders - Ergonomic construction of AST nodes
//
// Provides builder functions to make test code dramatically more concise:
//
// Before: Type::Reference(Box::new(Type::Vec(Box::new(Type::Int))))
// After:  type_ref(type_vec(Type::Int))
//
// Expected impact: 60-80% reduction in test code lines

use super::types::Type;
use super::ownership::OwnershipHint;
use super::literals::Literal;
use super::operators::{BinaryOp, CompoundOp, UnaryOp};
use super::core::{Expression, Parameter, Pattern, Statement};

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
pub fn param(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Inferred,
        is_mutable: false,
    }
}

/// Build reference parameter
pub fn param_ref(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Ref,
        is_mutable: false,
    }
}

/// Build mutable reference parameter
pub fn param_mut(name: impl Into<String>, type_: Type) -> Parameter {
    Parameter {
        name: name.into(),
        pattern: None,
        type_,
        ownership: OwnershipHint::Mut,
        is_mutable: true,
    }
}

/// Build owned parameter
pub fn param_owned(name: impl Into<String>, type_: Type) -> Parameter {
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
pub fn expr_int(value: i64) -> Expression {
    Expression::Literal {
        value: Literal::Int(value),
        location: None,
    }
}

/// Build float literal expression
pub fn expr_float(value: f64) -> Expression {
    Expression::Literal {
        value: Literal::Float(value),
        location: None,
    }
}

/// Build string literal expression
pub fn expr_string(value: impl Into<String>) -> Expression {
    Expression::Literal {
        value: Literal::String(value.into()),
        location: None,
    }
}

/// Build boolean literal expression
pub fn expr_bool(value: bool) -> Expression {
    Expression::Literal {
        value: Literal::Bool(value),
        location: None,
    }
}

/// Build character literal expression
pub fn expr_char(value: char) -> Expression {
    Expression::Literal {
        value: Literal::Char(value),
        location: None,
    }
}

/// Build variable/identifier expression
pub fn expr_var(name: impl Into<String>) -> Expression {
    Expression::Identifier {
        name: name.into(),
        location: None,
    }
}

/// Build binary operation expression
pub fn expr_binary(op: BinaryOp, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        left: Box::new(left),
        op,
        right: Box::new(right),
        location: None,
    }
}

/// Build addition expression (convenience)
pub fn expr_add(left: Expression, right: Expression) -> Expression {
    expr_binary(BinaryOp::Add, left, right)
}

/// Build subtraction expression (convenience)
pub fn expr_sub(left: Expression, right: Expression) -> Expression {
    expr_binary(BinaryOp::Sub, left, right)
}

/// Build multiplication expression (convenience)
pub fn expr_mul(left: Expression, right: Expression) -> Expression {
    expr_binary(BinaryOp::Mul, left, right)
}

/// Build division expression (convenience)
pub fn expr_div(left: Expression, right: Expression) -> Expression {
    expr_binary(BinaryOp::Div, left, right)
}

/// Build equality expression (convenience)
pub fn expr_eq(left: Expression, right: Expression) -> Expression {
    expr_binary(BinaryOp::Eq, left, right)
}

/// Build unary operation expression
pub fn expr_unary(op: UnaryOp, operand: Expression) -> Expression {
    Expression::Unary {
        op,
        operand: Box::new(operand),
        location: None,
    }
}

/// Build negation expression (convenience)
pub fn expr_neg(operand: Expression) -> Expression {
    expr_unary(UnaryOp::Neg, operand)
}

/// Build logical NOT expression (convenience)
pub fn expr_not(operand: Expression) -> Expression {
    expr_unary(UnaryOp::Not, operand)
}

/// Build function call expression
pub fn expr_call(function: impl Into<String>, arguments: Vec<Expression>) -> Expression {
    Expression::Call {
        function: Box::new(expr_var(function)),
        arguments: arguments.into_iter().map(|arg| (None, arg)).collect(),
        location: None,
    }
}

/// Build method call expression
pub fn expr_method(
    object: Expression,
    method: impl Into<String>,
    arguments: Vec<Expression>,
) -> Expression {
    Expression::MethodCall {
        object: Box::new(object),
        method: method.into(),
        type_args: None,
        arguments: arguments.into_iter().map(|arg| (None, arg)).collect(),
        location: None,
    }
}

/// Build field access expression
pub fn expr_field(object: Expression, field: impl Into<String>) -> Expression {
    Expression::FieldAccess {
        object: Box::new(object),
        field: field.into(),
        location: None,
    }
}

/// Build array index expression
pub fn expr_index(array: Expression, index: Expression) -> Expression {
    Expression::Index {
        object: Box::new(array),
        index: Box::new(index),
        location: None,
    }
}

/// Build array literal expression
pub fn expr_array(elements: Vec<Expression>) -> Expression {
    Expression::Array {
        elements,
        location: None,
    }
}

/// Build tuple expression
pub fn expr_tuple(elements: Vec<Expression>) -> Expression {
    Expression::Tuple {
        elements,
        location: None,
    }
}

/// Build block expression
pub fn expr_block(statements: Vec<Statement>) -> Expression {
    Expression::Block {
        statements,
        location: None,
    }
}

// ============================================================================
// STATEMENT BUILDERS
// ============================================================================

/// Build let statement (immutable)
pub fn stmt_let(name: impl Into<String>, type_: Option<Type>, value: Expression) -> Statement {
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
pub fn stmt_let_mut(name: impl Into<String>, type_: Option<Type>, value: Expression) -> Statement {
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
pub fn stmt_assign(target: Expression, value: Expression) -> Statement {
    Statement::Assignment {
        target,
        value,
        compound_op: None,
        location: None,
    }
}

/// Build compound assignment statement (+=, -=, etc.)
pub fn stmt_compound_assign(op: CompoundOp, target: Expression, value: Expression) -> Statement {
    Statement::Assignment {
        target,
        value,
        compound_op: Some(op),
        location: None,
    }
}

/// Build return statement
pub fn stmt_return(value: Option<Expression>) -> Statement {
    Statement::Return {
        value,
        location: None,
    }
}

/// Build expression statement
pub fn stmt_expr(expr: Expression) -> Statement {
    Statement::Expression {
        expr,
        location: None,
    }
}

/// Build if statement (no else)
pub fn stmt_if(
    condition: Expression,
    then_block: Vec<Statement>,
    else_block: Option<Vec<Statement>>,
) -> Statement {
    Statement::If {
        condition,
        then_block,
        else_block,
        location: None,
    }
}

/// Build if-else statement (convenience)
pub fn stmt_if_else(
    condition: Expression,
    then_block: Vec<Statement>,
    else_block: Vec<Statement>,
) -> Statement {
    Statement::If {
        condition,
        then_block,
        else_block: Some(else_block),
        location: None,
    }
}

/// Build while loop statement
pub fn stmt_while(condition: Expression, body: Vec<Statement>) -> Statement {
    Statement::While {
        condition,
        body,
        location: None,
    }
}

/// Build infinite loop statement
pub fn stmt_loop(body: Vec<Statement>) -> Statement {
    Statement::Loop {
        body,
        location: None,
    }
}

/// Build break statement
pub fn stmt_break() -> Statement {
    Statement::Break { location: None }
}

/// Build continue statement
pub fn stmt_continue() -> Statement {
    Statement::Continue { location: None }
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
        assert_eq!(
            type_ref(Type::Int),
            Type::Reference(Box::new(Type::Int))
        );
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
            Type::Option(Box::new(Type::MutableReference(Box::new(
                Type::Vec(Box::new(Type::String))
            ))))
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
        assert_eq!(
            param.type_,
            Type::Vec(Box::new(Type::Int))
        );
        assert_eq!(param.ownership, OwnershipHint::Ref);
    }
}

