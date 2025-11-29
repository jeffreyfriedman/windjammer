// UI Parser Extensions
//
// Additional parsing logic for UI-specific syntax

#![allow(clippy::collapsible_match)]

use crate::parser::ast::{Expression, FunctionDecl, Statement};
use crate::ui::ast_extensions::{EffectDecl, MemoDecl, SignalDecl};
// Parser for UI-specific constructs

/// Extract signal declarations from component body
pub fn extract_signals(func: &FunctionDecl) -> Vec<SignalDecl> {
    let mut signals = Vec::new();

    for stmt in &func.body {
        if let Statement::Let { pattern, value, .. } = stmt {
            // Extract name from pattern
            if let crate::parser::ast::Pattern::Identifier(name) = pattern {
                if let Expression::Call {
                    function,
                    arguments,
                    ..
                } = value
                {
                    // Check if function is "signal"
                    if let Expression::Identifier { name: fn_name, .. } = function.as_ref() {
                        if fn_name == "signal" && !arguments.is_empty() {
                            signals.push(SignalDecl {
                                name: name.clone(),
                                initial_value: arguments[0].1.clone(), // Get expression from tuple
                            });
                        }
                    }
                }
            }
        }
    }

    signals
}

/// Extract memo declarations from component body
pub fn extract_memos(func: &FunctionDecl) -> Vec<MemoDecl> {
    let mut memos = Vec::new();

    for stmt in &func.body {
        if let Statement::Let { pattern, value, .. } = stmt {
            // Extract name from pattern
            if let crate::parser::ast::Pattern::Identifier(name) = pattern {
                if let Expression::Call {
                    function,
                    arguments,
                    ..
                } = value
                {
                    // Check if function is "memo"
                    if let Expression::Identifier { name: fn_name, .. } = function.as_ref() {
                        if fn_name == "memo" && !arguments.is_empty() {
                            memos.push(MemoDecl {
                                name: name.clone(),
                                computation: arguments[0].1.clone(), // Get expression from tuple
                            });
                        }
                    }
                }
            }
        }
    }

    memos
}

/// Extract effect declarations from component body
pub fn extract_effects(func: &FunctionDecl) -> Vec<EffectDecl> {
    let mut effects = Vec::new();

    for stmt in &func.body {
        if let Statement::Expression { expr, .. } = stmt {
            if let Expression::Call {
                function,
                arguments,
                ..
            } = expr
            {
                // Check if function is "effect"
                if let Expression::Identifier { name: fn_name, .. } = function.as_ref() {
                    if fn_name == "effect" && !arguments.is_empty() {
                        effects.push(EffectDecl {
                            callback: arguments[0].1.clone(), // Get expression from tuple
                        });
                    }
                }
            }
        }
    }

    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Decorator, Expression, Statement, Type};

    fn create_test_component(body: Vec<Statement>) -> FunctionDecl {
        FunctionDecl {
            is_pub: false,
            is_extern: false,
            name: "TestComponent".to_string(),
            type_params: vec![],
            where_clause: vec![],
            decorators: vec![Decorator {
                name: "component".to_string(),
                arguments: vec![],
            }],
            is_async: false,
            parameters: vec![],
            return_type: Some(Type::Custom("UI".to_string())),
            body,
            parent_type: None,
        }
    }

    #[test]
    fn test_extract_signals_empty() {
        let func = create_test_component(vec![]);
        let signals = extract_signals(&func);
        assert_eq!(signals.len(), 0);
    }

    #[test]
    fn test_extract_signals_single() {
        let func = create_test_component(vec![Statement::Let {
            pattern: crate::parser::ast::Pattern::Identifier("count".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "signal".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Literal {
                        value: crate::parser::ast::Literal::Int(0),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }]);

        let signals = extract_signals(&func);
        assert_eq!(signals.len(), 1);
        assert_eq!(signals[0].name, "count");
    }

    #[test]
    fn test_extract_signals_multiple() {
        let func = create_test_component(vec![
            Statement::Let {
                pattern: crate::parser::ast::Pattern::Identifier("count".to_string()),
                mutable: false,
                type_: None,
                value: Expression::Call {
                    function: Box::new(Expression::Identifier {
                        name: "signal".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        Expression::Literal {
                            value: crate::parser::ast::Literal::Int(0),
                            location: None,
                        },
                    )],
                    location: None,
                },
                location: None,
            },
            Statement::Let {
                pattern: crate::parser::ast::Pattern::Identifier("name".to_string()),
                mutable: false,
                type_: None,
                value: Expression::Call {
                    function: Box::new(Expression::Identifier {
                        name: "signal".to_string(),
                        location: None,
                    }),
                    arguments: vec![(
                        None,
                        Expression::Literal {
                            value: crate::parser::ast::Literal::String("Alice".to_string()),
                            location: None,
                        },
                    )],
                    location: None,
                },
                location: None,
            },
        ]);

        let signals = extract_signals(&func);
        assert_eq!(signals.len(), 2);
        assert_eq!(signals[0].name, "count");
        assert_eq!(signals[1].name, "name");
    }

    #[test]
    fn test_extract_memos_empty() {
        let func = create_test_component(vec![]);
        let memos = extract_memos(&func);
        assert_eq!(memos.len(), 0);
    }

    #[test]
    fn test_extract_memos_single() {
        let func = create_test_component(vec![Statement::Let {
            pattern: crate::parser::ast::Pattern::Identifier("doubled".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "memo".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Closure {
                        parameters: vec![],
                        body: Box::new(Expression::Literal {
                            value: crate::parser::ast::Literal::Int(0),
                            location: None,
                        }),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }]);

        let memos = extract_memos(&func);
        assert_eq!(memos.len(), 1);
        assert_eq!(memos[0].name, "doubled");
    }

    #[test]
    fn test_extract_effects_empty() {
        let func = create_test_component(vec![]);
        let effects = extract_effects(&func);
        assert_eq!(effects.len(), 0);
    }

    #[test]
    fn test_extract_effects_single() {
        let func = create_test_component(vec![Statement::Expression {
            expr: Expression::Call {
                function: Box::new(Expression::Identifier {
                    name: "effect".to_string(),
                    location: None,
                }),
                arguments: vec![(
                    None,
                    Expression::Closure {
                        parameters: vec![],
                        body: Box::new(Expression::Literal {
                            value: crate::parser::ast::Literal::Int(0),
                            location: None,
                        }),
                        location: None,
                    },
                )],
                location: None,
            },
            location: None,
        }]);

        let effects = extract_effects(&func);
        assert_eq!(effects.len(), 1);
    }

    #[test]
    fn test_extract_non_signal_lets() {
        // Regular let statements should not be extracted as signals
        let func = create_test_component(vec![Statement::Let {
            pattern: crate::parser::ast::Pattern::Identifier("regular_var".to_string()),
            mutable: false,
            type_: None,
            value: Expression::Literal {
                value: crate::parser::ast::Literal::Int(42),
                location: None,
            },
            location: None,
        }]);

        let signals = extract_signals(&func);
        assert_eq!(signals.len(), 0);
    }
}
