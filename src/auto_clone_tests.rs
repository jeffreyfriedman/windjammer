//! Unit tests for [`super`] (`auto_clone`), kept in a sibling file so the analysis stays readable.

use super::*;
use crate::test_utils::{test_alloc_expr, test_alloc_stmt};

#[test]
fn test_simple_move_and_reuse() {
    // let x = vec![1, 2, 3]
    // takes_ownership(x)  // <- Should insert .clone() here
    // println!("{}", x.len())

    let func = FunctionDecl {
        name: "test".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("x".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Array {
                    elements: vec![
                        test_alloc_expr(Expression::Literal {
                            value: Literal::Int(1),
                            location: None,
                        }),
                        test_alloc_expr(Expression::Literal {
                            value: Literal::Int(2),
                            location: None,
                        }),
                        test_alloc_expr(Expression::Literal {
                            value: Literal::Int(3),
                            location: None,
                        }),
                    ],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "takes_ownership".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "x".to_string(),
                        location: None,
                    }),
                    method: "len".to_string(),
                    arguments: vec![],
                    type_args: None,
                    location: None,
                }),
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);

    // Should detect that x needs to be cloned at statement 1 (the function call)
    assert!(analysis.needs_clone("x", 1).is_some());
    assert_eq!(
        analysis.needs_clone("x", 1),
        Some(&CloneReason::MovedButUsedLater)
    );
}

#[test]
fn match_ok_binding_reuse_in_arm_block_needs_clone() {
    // P3.574: Ok(expr) => { takes(expr); takes(expr) } must clone the first move.
    let arm_block = test_alloc_expr(Expression::Block {
        statements: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("a".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "takes".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "expr".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("b".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "takes".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "expr".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::Binary {
                    left: test_alloc_expr(Expression::Identifier {
                        name: "a".to_string(),
                        location: None,
                    }),
                    op: crate::parser::BinaryOp::And,
                    right: test_alloc_expr(Expression::Identifier {
                        name: "b".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                location: None,
            }),
        ],
        location: None,
        is_unsafe: false,
    });
    let func = FunctionDecl {
        name: "test_reuse".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![test_alloc_stmt(Statement::Match {
            value: test_alloc_expr(Expression::Call {
                function: test_alloc_expr(Expression::Identifier {
                    name: "parse".to_string(),
                    location: None,
                }),
                arguments: vec![],
                location: None,
            }),
            arms: vec![
                crate::parser::MatchArm {
                    pattern: Pattern::EnumVariant(
                        "Ok".to_string(),
                        crate::parser::EnumPatternBinding::Single("expr".to_string()),
                    ),
                    guard: None,
                    body: arm_block,
                },
                crate::parser::MatchArm {
                    pattern: Pattern::EnumVariant(
                        "Err".to_string(),
                        crate::parser::EnumPatternBinding::Wildcard,
                    ),
                    guard: None,
                    body: test_alloc_expr(Expression::Literal {
                        value: Literal::Bool(false),
                        location: None,
                    }),
                },
            ],
            location: None,
        })],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone_anywhere("expr"),
        "P3.574: Ok(expr) arm reuse must schedule clone; sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn p3642_nested_match_scrutinee_arg_reuse_under_if_needs_clone() {
    // export shape: if … { let snapshot = …; match decode(snapshot) { Ok => {
    //   match other { Ok => { match decode(snapshot) … } } } } }
    let inner_decode = test_alloc_stmt(Statement::Match {
        value: test_alloc_expr(Expression::Call {
            function: test_alloc_expr(Expression::Identifier {
                name: "decode_store".to_string(),
                location: None,
            }),
            arguments: vec![(
                None,
                test_alloc_expr(Expression::Identifier {
                    name: "snapshot".to_string(),
                    location: None,
                }),
            )],
            location: None,
        }),
        arms: vec![crate::parser::MatchArm {
            pattern: Pattern::EnumVariant(
                "Ok".to_string(),
                crate::parser::EnumPatternBinding::Wildcard,
            ),
            guard: None,
            body: test_alloc_expr(Expression::Literal {
                value: Literal::Bool(true),
                location: None,
            }),
        }],
        location: None,
    });
    let middle_match = test_alloc_stmt(Statement::Match {
        value: test_alloc_expr(Expression::Call {
            function: test_alloc_expr(Expression::Identifier {
                name: "todos_to_json".to_string(),
                location: None,
            }),
            arguments: vec![],
            location: None,
        }),
        arms: vec![crate::parser::MatchArm {
            pattern: Pattern::EnumVariant(
                "Ok".to_string(),
                crate::parser::EnumPatternBinding::Single("body".to_string()),
            ),
            guard: None,
            body: test_alloc_expr(Expression::Block {
                statements: vec![inner_decode],
                location: None,
                is_unsafe: false,
            }),
        }],
        location: None,
    });
    let outer_match = test_alloc_stmt(Statement::Match {
        value: test_alloc_expr(Expression::Call {
            function: test_alloc_expr(Expression::Identifier {
                name: "decode_store".to_string(),
                location: None,
            }),
            arguments: vec![(
                None,
                test_alloc_expr(Expression::Identifier {
                    name: "snapshot".to_string(),
                    location: None,
                }),
            )],
            location: None,
        }),
        arms: vec![crate::parser::MatchArm {
            pattern: Pattern::EnumVariant(
                "Ok".to_string(),
                crate::parser::EnumPatternBinding::Single("copy".to_string()),
            ),
            guard: None,
            body: test_alloc_expr(Expression::Block {
                statements: vec![middle_match],
                location: None,
                is_unsafe: false,
            }),
        }],
        location: None,
    });
    let func = FunctionDecl {
        name: "export".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("snapshot".to_string()),
                mutable: false,
                type_: Some(Type::String),
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "encode_store".to_string(),
                        location: None,
                    }),
                    arguments: vec![],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::If {
                condition: test_alloc_expr(Expression::Literal {
                    value: Literal::Bool(true),
                    location: None,
                }),
                then_block: vec![outer_match],
                else_block: None,
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone_anywhere("snapshot"),
        "P3.642: first decode_store(snapshot) under nested match must clone; sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn test_owned_field_extract_value_param_still_needs_clone_on_reuse() {
    // regression-063: value_tag(value: Value) match-projects but emits owned Value —
    // field_extract must not demote Move→Read; first call needs .clone().
    let mut registry = crate::analyzer::SignatureRegistry::new();
    let mut sig = crate::analyzer::FunctionSignature {
        name: "value_tag".to_string(),
        param_types: vec![Type::Custom("Value".to_string())],
        formal_param_types: vec![Type::Custom("Value".to_string())],
        param_ownership: vec![crate::analyzer::OwnershipMode::Borrowed],
        return_type: Some(Type::Int32),
        return_ownership: crate::analyzer::OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: Some(vec![false]),
        string_ref_string_formal_params: None,
        field_extract_params: Some(vec![true]),
        forwarding_borrow_params: None,
    };
    registry.add_function("value_tag".to_string(), sig.clone());
    sig.name = "value_i64".to_string();
    registry.add_function("value_i64".to_string(), sig);

    let func = FunctionDecl {
        name: "seed_write".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "value".to_string(),
            pattern: None,
            type_: Type::Custom("Value".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("tag".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "value_tag".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "value".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("payload".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "value_i64".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "value".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
    assert!(
        analysis.needs_clone("value", 0).is_some(),
        "owned field-extract formal must still move — clone at first use"
    );
}

#[test]
fn test_param_let_alias_then_reuse_needs_clone_at_alias() {
    let func = FunctionDecl {
        name: "send".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "message".to_string(),
            pattern: None,
            type_: Type::Custom("Message".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("msg_copy".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Identifier {
                    name: "message".to_string(),
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "msg_copy".to_string(),
                        location: None,
                    }),
                    method: "use_it".to_string(),
                    arguments: vec![],
                    type_args: None,
                    location: None,
                }),
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "push".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "message".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone("message", 0).is_some(),
        "let msg_copy = message moves param; push(message) later needs clone at let"
    );
}

#[test]
fn test_no_clone_needed_single_use() {
    // let x = vec![1, 2, 3]
    // takes_ownership(x)  // <- No clone needed, x not used again

    let func = FunctionDecl {
        name: "test".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("x".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Array {
                    elements: vec![],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "takes_ownership".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);

    // Should NOT detect any clones needed
    assert!(analysis.needs_clone("x", 1).is_none());
}

#[test]
fn test_distinct_owned_field_moves_do_not_clone() {
    // return_f64(buf.scores); return_f64(buf.next) — distinct fields.
    let func = FunctionDecl {
        name: "release".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "buf".to_string(),
            pattern: None,
            type_: Type::Custom("Buf".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "return_f64".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "buf".to_string(),
                                location: None,
                            }),
                            field: "scores".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "return_f64".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "buf".to_string(),
                                location: None,
                            }),
                            field: "next".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone("buf.scores", 0).is_none(),
        "distinct field move must not clone earlier field"
    );
    assert!(
        analysis.needs_clone("buf.next", 1).is_none(),
        "distinct field move must not clone later field"
    );
}

#[test]
fn test_self_field_in_if_expr_inside_while_loop_needs_clone() {
    // Reproduces rating.wj bug:
    //   while i <= self.max {
    //       let star_color = if filled { self.color } else { "#e2e8f0" }
    //       html.push_str(star_color)
    //       i += 1
    //   }
    // self.color is a String field used in a while loop if-expression.
    // It must be cloned because the loop executes multiple times.

    let func = FunctionDecl {
        name: "render".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "self".to_string(),
            pattern: None,
            type_: Type::Custom("Rating".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: Some(Type::String),
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: Some("Rating".to_string()),
        impl_trait: None,
        doc_comment: None,
        body: vec![
            // let mut i = 1
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("i".to_string()),
                mutable: true,
                type_: None,
                value: test_alloc_expr(Expression::Literal {
                    value: Literal::Int(1),
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            // while i <= self.max {
            test_alloc_stmt(Statement::While {
                label: None,
                condition: test_alloc_expr(Expression::Binary {
                    left: test_alloc_expr(Expression::Identifier {
                        name: "i".to_string(),
                        location: None,
                    }),
                    op: BinaryOp::Le,
                    right: test_alloc_expr(Expression::FieldAccess {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "self".to_string(),
                            location: None,
                        }),
                        field: "max".to_string(),
                        location: None,
                    }),
                    location: None,
                }),
                label: None,
                body: vec![
                    // let star_color = if filled { self.color } else { "#e2e8f0" }
                    test_alloc_stmt(Statement::Let {
                        pattern: Pattern::Identifier("star_color".to_string()),
                        mutable: false,
                        type_: None,
                        value: test_alloc_expr(Expression::Block {
                            is_unsafe: false,
                            statements: vec![test_alloc_stmt(Statement::If {
                                condition: test_alloc_expr(Expression::Identifier {
                                    name: "filled".to_string(),
                                    location: None,
                                }),
                                then_block: vec![test_alloc_stmt(Statement::Expression {
                                    expr: test_alloc_expr(Expression::FieldAccess {
                                        object: test_alloc_expr(Expression::Identifier {
                                            name: "self".to_string(),
                                            location: None,
                                        }),
                                        field: "color".to_string(),
                                        location: None,
                                    }),
                                    location: None,
                                })],
                                else_block: Some(vec![test_alloc_stmt(
                                    Statement::Expression {
                                        expr: test_alloc_expr(Expression::Literal {
                                            value: Literal::String("#e2e8f0".to_string()),
                                            location: None,
                                        }),
                                        location: None,
                                    },
                                )]),
                                location: None,
                            })],
                            location: None,
                        }),
                        else_block: None,
                        location: None,
                    }),
                    // i += 1
                    test_alloc_stmt(Statement::Assignment {
                        target: test_alloc_expr(Expression::Identifier {
                            name: "i".to_string(),
                            location: None,
                        }),
                        value: test_alloc_expr(Expression::Binary {
                            left: test_alloc_expr(Expression::Identifier {
                                name: "i".to_string(),
                                location: None,
                            }),
                            op: BinaryOp::Add,
                            right: test_alloc_expr(Expression::Literal {
                                value: Literal::Int(1),
                                location: None,
                            }),
                            location: None,
                        }),
                        compound_op: None,
                        location: None,
                    }),
                ],
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);

    // self.color is used inside a while loop but defined outside (it's a field
    // of self). The loop executes multiple times, so each iteration would try
    // to move self.color. Auto-clone must detect this.
    assert!(
        analysis.needs_clone_anywhere("self.color"),
        "self.color in while loop if-expression must be flagged for clone"
    );
}

#[test]
fn test_owned_field_extract_without_emitted_flags_still_moves() {
    let mut registry = crate::analyzer::SignatureRegistry::new();
    let sig = crate::analyzer::FunctionSignature {
        name: "value_tag".to_string(),
        param_types: vec![Type::Reference(Box::new(Type::Custom("Value".to_string())))],
        formal_param_types: vec![Type::Custom("Value".to_string())],
        param_ownership: vec![crate::analyzer::OwnershipMode::Borrowed],
        return_type: Some(Type::Int32),
        return_ownership: crate::analyzer::OwnershipMode::Owned,
        has_self_receiver: false,
        is_extern: false,
        emitted_rust_ref_params: None,
        string_ref_string_formal_params: None,
        field_extract_params: Some(vec![true]),
        forwarding_borrow_params: None,
    };
    registry.add_function("value_tag".to_string(), sig.clone());
    let mut sig2 = sig.clone();
    sig2.name = "value_i64".to_string();
    registry.add_function("value_i64".to_string(), sig2);

    let func = FunctionDecl {
        name: "seed_write".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "value".to_string(),
            pattern: None,
            type_: Type::Custom("Value".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("tag".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "value_tag".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "value".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("payload".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "value_i64".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "value".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
    assert!(
        analysis.needs_clone("value", 0).is_some(),
        "bare WJ formal + Reference param_types must still move; got sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn test_seed_write_value_reuse_needs_clone_without_registry() {
    let func = FunctionDecl {
        name: "seed_write".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "value".to_string(),
            pattern: None,
            type_: Type::Custom("Value".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("tag".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "value_tag".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "value".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("payload".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "value_i64".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "value".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };
    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone("value", 0).is_some(),
        "seed_write without registry must still clone; sites={:?}",
        analysis.clone_sites
    );
}

/// P3.647: recursive owned Vec formal reused across two method calls.
#[test]
fn p3647_recursive_owned_vec_param_needs_clone() {
    let func = FunctionDecl {
        name: "evaluate_node".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![
            Parameter {
                name: "self".to_string(),
                pattern: None,
                type_: Type::Custom("Tree".to_string()),
                ownership: OwnershipHint::Ref,
                is_mutable: false,
                decorators: vec![],
            },
            Parameter {
                name: "node_id".to_string(),
                pattern: None,
                type_: Type::Custom("u32".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
            Parameter {
                name: "clips".to_string(),
                pattern: None,
                type_: Type::Vec(Box::new(Type::Custom("Clip".to_string()))),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
            Parameter {
                name: "t".to_string(),
                pattern: None,
                type_: Type::Float,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
        ],
        return_type: Some(Type::Custom("u32".to_string())),
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: Some("Tree".to_string()),
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("a".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "self".to_string(),
                        location: None,
                    }),
                    method: "evaluate_node".to_string(),
                    type_args: None,
                    arguments: vec![
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "node_id".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "clips".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "t".to_string(),
                                location: None,
                            }),
                        ),
                    ],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("b".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "self".to_string(),
                        location: None,
                    }),
                    method: "evaluate_node".to_string(),
                    type_args: None,
                    arguments: vec![
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "node_id".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "clips".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "t".to_string(),
                                location: None,
                            }),
                        ),
                    ],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };
    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone("clips", 0).is_some(),
        "P3.647: first recursive move of owned Vec must clone; sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn p3647_parsed_source_needs_clone_on_first_recursive_clips() {
    use crate::analyzer::Analyzer;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    let source = r#"
pub struct Clip { pub id: u32 }
pub struct Tree { pub root: u32 }
impl Tree {
pub fn evaluate(self, clips: Vec<Clip>, t: f32) -> u32 {
    self.evaluate_node(self.root, clips, t)
}
fn evaluate_node(self, node_id: u32, clips: Vec<Clip>, t: f32) -> u32 {
    if node_id == 0 {
        return clips[0].id
    }
    let a = self.evaluate_node(node_id - 1, clips, t)
    let b = self.evaluate_node(node_id - 1, clips, t)
    a + b
}
}
"#;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut analyzer = Analyzer::new();
    let (analyzed, registry, _) = analyzer.analyze_program(&program).unwrap();
    let func = analyzed
        .iter()
        .find(|f| f.decl.name == "evaluate_node")
        .expect("evaluate_node");
    let analysis =
        AutoCloneAnalysis::analyze_function_with_registry(&func.decl, Some(&registry));
    assert_eq!(
        AutoCloneAnalysis::method_call_arg_usage_kind("evaluate_node", 1, 3, Some(&registry),),
        UsageKind::Move,
        "P3.647: bare owned Vec formal must Move despite analyzer Borrowed"
    );
    assert!(
        analysis.needs_clone_anywhere("clips")
            || func.auto_clone_analysis.needs_clone_anywhere("clips"),
        "P3.647: clips must need clone; sites={:?} / {:?}",
        analysis.clone_sites,
        func.auto_clone_analysis.clone_sites
    );
}

fn test_multi_use_owned_string_param_needs_clone_on_first_call() {
    let func = FunctionDecl {
        name: "render_panel".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![
            Parameter {
                name: "title".to_string(),
                pattern: None,
                type_: Type::String,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
            Parameter {
                name: "blurb".to_string(),
                pattern: None,
                type_: Type::String,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
        ],
        return_type: Some(Type::String),
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("crumbs".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "crumbs_for".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "title".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("title_e".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "escape_html".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "title".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone("title", 0).is_some(),
        "multi-use owned string param must clone at first move; sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn test_multi_use_owned_string_param_with_global_registry_still_needs_clone() {
    let mut registry = crate::analyzer::SignatureRegistry::new();
    for name in [
        "escape_html",
        "html::escape_html",
        "crumbs_for",
        "html::crumbs_for",
    ] {
        registry.add_function(
            name.to_string(),
            crate::analyzer::FunctionSignature {
                name: name.rsplit("::").next().unwrap().to_string(),
                param_types: vec![Type::Reference(Box::new(Type::String))],
                formal_param_types: vec![Type::String],
                param_ownership: vec![crate::analyzer::OwnershipMode::Borrowed],
                return_type: Some(Type::String),
                return_ownership: crate::analyzer::OwnershipMode::Owned,
                has_self_receiver: false,
                is_extern: false,
                emitted_rust_ref_params: Some(vec![true]),
                string_ref_string_formal_params: None,
                field_extract_params: None,
                forwarding_borrow_params: None,
            },
        );
    }

    let func = FunctionDecl {
        name: "render_panel".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![Parameter {
            name: "title".to_string(),
            pattern: None,
            type_: Type::String,
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: Some(Type::String),
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("crumbs".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "crumbs_for".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "title".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("title_e".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "escape_html".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "title".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
    assert!(
        analysis.needs_clone("title", 0).is_some(),
        "stale borrowed callee metadata must not suppress auto-clone; sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn trait_owned_get_arg_not_treated_as_map_key_borrow() {
    // `StorageEngine::get(txn: Txn, key)` must not inherit HashMap::get's map-key
    // Read heuristic — first call moves `txn`, second needs `.clone()`.
    let mut registry = crate::analyzer::SignatureRegistry::new();
    // Homonym: stdlib-style borrowed map key.
    registry.add_function(
        "HashMap::get".to_string(),
        crate::analyzer::FunctionSignature {
            name: "get".to_string(),
            param_types: vec![
                crate::parser::Type::Reference(Box::new(crate::parser::Type::Custom(
                    "HashMap".to_string(),
                ))),
                crate::parser::Type::Reference(Box::new(crate::parser::Type::String)),
            ],
            formal_param_types: vec![
                crate::parser::Type::Custom("HashMap".to_string()),
                crate::parser::Type::String,
            ],
            param_ownership: vec![
                crate::analyzer::OwnershipMode::Borrowed,
                crate::analyzer::OwnershipMode::Borrowed,
            ],
            return_type: None,
            return_ownership: crate::analyzer::OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true, true]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );
    registry.add_function(
        "StorageEngine::get".to_string(),
        crate::analyzer::FunctionSignature {
            name: "get".to_string(),
            param_types: vec![
                crate::parser::Type::Reference(Box::new(crate::parser::Type::Custom(
                    "StorageEngine".to_string(),
                ))),
                crate::parser::Type::Custom("Txn".to_string()),
                crate::parser::Type::String,
            ],
            formal_param_types: vec![
                crate::parser::Type::Custom("StorageEngine".to_string()),
                crate::parser::Type::Custom("Txn".to_string()),
                crate::parser::Type::String,
            ],
            param_ownership: vec![
                crate::analyzer::OwnershipMode::Borrowed,
                crate::analyzer::OwnershipMode::Owned,
                crate::analyzer::OwnershipMode::Owned,
            ],
            return_type: None,
            return_ownership: crate::analyzer::OwnershipMode::Owned,
            has_self_receiver: true,
            is_extern: false,
            emitted_rust_ref_params: Some(vec![true, false, false]),
            string_ref_string_formal_params: None,
            field_extract_params: None,
            forwarding_borrow_params: None,
        },
    );

    let func = FunctionDecl {
        name: "read_twice".to_string(),
        is_pub: false,
        is_extern: false,
        parameters: vec![
            Parameter {
                name: "engine".to_string(),
                pattern: None,
                type_: Type::Custom("MemoryEngine".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
            Parameter {
                name: "txn".to_string(),
                pattern: None,
                type_: Type::Custom("Txn".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
        ],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("a".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "engine".to_string(),
                        location: None,
                    }),
                    method: "get".to_string(),
                    arguments: vec![
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "txn".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            None,
                            test_alloc_expr(Expression::Literal {
                                value: Literal::String("a".to_string()),
                                location: None,
                            }),
                        ),
                    ],
                    type_args: None,
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("b".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "engine".to_string(),
                        location: None,
                    }),
                    method: "get".to_string(),
                    arguments: vec![
                        (
                            None,
                            test_alloc_expr(Expression::Identifier {
                                name: "txn".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            None,
                            test_alloc_expr(Expression::Literal {
                                value: Literal::String("b".to_string()),
                                location: None,
                            }),
                        ),
                    ],
                    type_args: None,
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };

    let analysis = AutoCloneAnalysis::analyze_function_with_registry(&func, Some(&registry));
    assert!(
        analysis.needs_clone("txn", 0).is_some(),
        "owned trait get arg must clone on reuse despite HashMap::get homonym; sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn test_field_move_then_format_reuse_needs_clone() {
    // label_for shape: escape_html(row.account_code); format!(..., row.account_code, ...)
    let func = FunctionDecl {
        name: "label_for".to_string(),
        is_pub: true,
        is_extern: false,
        parameters: vec![crate::parser::Parameter {
            name: "row".to_string(),
            pattern: None,
            type_: Type::Custom("Row".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: Some(Type::String),
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: None,
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("code".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::Identifier {
                        name: "escape_html".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "row".to_string(),
                                location: None,
                            }),
                            field: "account_code".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("label_raw".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::MacroInvocation {
                    name: "format".to_string(),
                    args: vec![
                        test_alloc_expr(Expression::Literal {
                            value: Literal::String("{} · {}".to_string()),
                            location: None,
                        }),
                        test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "row".to_string(),
                                location: None,
                            }),
                            field: "account_code".to_string(),
                            location: None,
                        }),
                        test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "row".to_string(),
                                location: None,
                            }),
                            field: "account_name".to_string(),
                            location: None,
                        }),
                    ],
                    delimiter: MacroDelimiter::Parens,
                    is_repeat: false,
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
        ],
    };
    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone_anywhere("row.account_code"),
        "expected clone site for row.account_code, sites={:?}",
        analysis.clone_sites
    );
    assert!(
        analysis.needs_clone("row.account_code", 0).is_some(),
        "first move into owned formal must clone when field is reused"
    );
}

#[test]
fn owned_self_wither_must_not_clone_distinct_fields() {
    // `let mut items = self.items; items.push(x); Builder { graph, items, label }`
    // Owned self may move distinct fields. `&self` getters still always-clone.
    let func = FunctionDecl {
        name: "push".to_string(),
        is_pub: true,
        is_extern: false,
        parameters: vec![
            Parameter {
                name: "self".to_string(),
                pattern: None,
                type_: Type::Custom("Builder".to_string()),
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
            Parameter {
                name: "x".to_string(),
                pattern: None,
                type_: Type::Int32,
                ownership: OwnershipHint::Owned,
                is_mutable: false,
                decorators: vec![],
            },
        ],
        return_type: Some(Type::Custom("Builder".to_string())),
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: Some("Builder".to_string()),
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("items".to_string()),
                mutable: true,
                type_: None,
                value: test_alloc_expr(Expression::FieldAccess {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "self".to_string(),
                        location: None,
                    }),
                    field: "items".to_string(),
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Expression {
                expr: test_alloc_expr(Expression::MethodCall {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "items".to_string(),
                        location: None,
                    }),
                    method: "push".to_string(),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::Identifier {
                            name: "x".to_string(),
                            location: None,
                        }),
                    )],
                    type_args: None,
                    location: None,
                }),
                location: None,
            }),
            test_alloc_stmt(Statement::Return {
                value: Some(test_alloc_expr(Expression::StructLiteral {
                    name: "Builder".to_string(),
                    fields: vec![
                        (
                            "graph".to_string(),
                            test_alloc_expr(Expression::FieldAccess {
                                object: test_alloc_expr(Expression::Identifier {
                                    name: "self".to_string(),
                                    location: None,
                                }),
                                field: "graph".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            "items".to_string(),
                            test_alloc_expr(Expression::Identifier {
                                name: "items".to_string(),
                                location: None,
                            }),
                        ),
                        (
                            "label".to_string(),
                            test_alloc_expr(Expression::FieldAccess {
                                object: test_alloc_expr(Expression::Identifier {
                                    name: "self".to_string(),
                                    location: None,
                                }),
                                field: "label".to_string(),
                                location: None,
                            }),
                        ),
                    ],
                    location: None,
                })),
                location: None,
            }),
        ],
    };
    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        analysis.needs_clone("self.items", 0).is_none(),
        "owned-self wither must move self.items, sites={:?}",
        analysis.clone_sites
    );
    assert!(
        !analysis.needs_clone_anywhere("self.graph"),
        "owned-self wither must move self.graph, sites={:?}",
        analysis.clone_sites
    );
    assert!(
        !analysis.needs_clone_anywhere("self.label"),
        "owned-self wither must move self.label, sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn owned_self_ctor_arg_must_not_clone_moved_field() {
    // `let v = Vox::new(self.scene); self.grid = 1`
    let func = FunctionDecl {
        name: "initialize".to_string(),
        is_pub: true,
        is_extern: false,
        parameters: vec![Parameter {
            name: "self".to_string(),
            pattern: None,
            type_: Type::Custom("Demo".to_string()),
            ownership: OwnershipHint::Owned,
            is_mutable: false,
            decorators: vec![],
        }],
        return_type: None,
        return_decorators: Vec::new(),
        type_params: vec![],
        where_clause: vec![],
        decorators: vec![],
        is_async: false,
        parent_type: Some("Demo".to_string()),
        impl_trait: None,
        doc_comment: None,
        body: vec![
            test_alloc_stmt(Statement::Let {
                pattern: Pattern::Identifier("v".to_string()),
                mutable: false,
                type_: None,
                value: test_alloc_expr(Expression::Call {
                    function: test_alloc_expr(Expression::FieldAccess {
                        object: test_alloc_expr(Expression::Identifier {
                            name: "Vox".to_string(),
                            location: None,
                        }),
                        field: "new".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        test_alloc_expr(Expression::FieldAccess {
                            object: test_alloc_expr(Expression::Identifier {
                                name: "self".to_string(),
                                location: None,
                            }),
                            field: "scene".to_string(),
                            location: None,
                        }),
                    )],
                    location: None,
                }),
                else_block: None,
                location: None,
            }),
            test_alloc_stmt(Statement::Assignment {
                target: test_alloc_expr(Expression::FieldAccess {
                    object: test_alloc_expr(Expression::Identifier {
                        name: "self".to_string(),
                        location: None,
                    }),
                    field: "grid".to_string(),
                    location: None,
                }),
                value: test_alloc_expr(Expression::Literal {
                    value: Literal::Int(1),
                    location: None,
                }),
                compound_op: None,
                location: None,
            }),
        ],
    };
    let analysis = AutoCloneAnalysis::analyze_function(&func);
    assert!(
        !analysis.needs_clone_anywhere("self.scene"),
        "owned-self ctor arg must move self.scene, sites={:?}",
        analysis.clone_sites
    );
}

#[test]
fn exclusive_if_let_return_does_not_clone_string() {
    let src = r#"
pub fn node_text(node: Node) -> string { node.body }
pub fn substitute(text: string, world: World) -> string { text }
pub fn current_text(node: Option<Node>, world: Option<World>) -> string {
if let Some(n) = node {
    let text = node_text(n)
    if let Some(w) = world {
        return substitute(text, w)
    }
    return text
}
""
}
"#;
    let mut lexer = crate::lexer::Lexer::new(src);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = crate::parser::Parser::new(tokens);
    let program = parser.parse().expect("parse");
    let func = program
        .items
        .iter()
        .find_map(|item| match item {
            Item::Function { decl, .. } if decl.name == "current_text" => Some(decl),
            _ => None,
        })
        .expect("current_text");
    let analysis = AutoCloneAnalysis::analyze_function(func);
    assert!(
        !analysis.needs_clone_anywhere("text"),
        "exclusive if-let arms must not clone text, sites={:?}",
        analysis.clone_sites
    );
}
