/// String Parameter Optimization Analyzer
///
/// Phase 2 of the String Parameter Optimization plan:
/// Analyzes function bodies to determine if borrowed string parameters can safely
/// use `&str` instead of `&String`.
///
/// PROPER APPROACH (No String Matching!):
/// Instead of hard-coding method names, we analyze method signatures:
/// 1. Look up the method in the type registry
/// 2. Check if any parameter types are `&String` or `&T where T: Borrow<String>`
/// 3. If a parameter is passed to such a method → use &String (correctness)
/// 4. Otherwise → use &str (performance)
///
/// This is:
/// - Extensible: Works with custom methods automatically
/// - Maintainable: No hard-coded string lists
/// - Correct: Based on actual type system, not heuristics
///
/// PHASE 2 MVP: Conservative implementation - returns empty set (uses &String everywhere)
/// Full implementation requires:
/// 1. Method signature lookup in type registry
/// 2. AST traversal to find method calls
/// 3. Parameter flow analysis (which params are passed where)

use crate::analyzer::Analyzer;
use crate::parser::{Expression, FunctionDecl, Statement, Type};
use std::collections::HashSet;

impl<'ast> Analyzer<'ast> {
    /// Analyze all string parameters in a function and return the set that can use &str
    ///
    /// THE PROPER WAY:
    /// - Traverse function body AST
    /// - For each method call, look up its signature in type registry
    /// - Check if any method parameter expects &String (not &str)
    /// - If parameter flows to such a method → must use &String
    /// - Otherwise → can safely use &str
    ///
    /// PHASE 2 MVP: Conservative - returns empty set
    /// This maintains Phase 1 baseline (&String everywhere) until full analysis is implemented
    pub fn analyze_str_ref_optimizable_params(&self, func: &FunctionDecl) -> HashSet<String> {
        // Phase 2 MVP: Conservative - no optimization yet
        // Returns empty set → all borrowed string params use &String
        
        // TODO: Implement proper type-based analysis:
        // 1. Walk function body statements
        // 2. For each method call expression:
        //    - Look up receiver type (Vec<T>, HashMap<K,V>, etc.)
        //    - Look up method signature from type registry
        //    - Check parameter types for &String
        // 3. Track which function parameters flow to those methods
        // 4. Return set of params that DON'T flow to &String methods
        
        let _body = &func.body; // Will use this in full implementation
        
        HashSet::new()
    }

    /// Helper: Check if an expression is a method call that needs &String
    /// 
    /// THE PROPER WAY: Look up method signature, check parameter types
    /// NO STRING MATCHING!
    #[allow(dead_code)]
    fn expr_needs_string_ref(&self, _expr: &Expression) -> bool {
        // TODO: Implement with type registry lookup
        // 
        // Example proper implementation:
        // match expr {
        //     Expression::MethodCall { object, method, .. } => {
        //         let receiver_type = self.infer_expression_type(object);
        //         if let Some(method_sig) = self.lookup_method_signature(&receiver_type, method) {
        //             // Check if ANY parameter is &String (not &str)
        //             return method_sig.parameters.iter().any(|p| {
        //                 matches!(p.type_, Type::Reference(box Type::String))
        //             });
        //         }
        //     }
        //     _ => {}
        // }
        false
    }

    /// Helper: Check if a statement contains method calls needing &String
    #[allow(dead_code)]
    fn statement_needs_string_ref(&self, _stmt: &Statement) -> bool {
        // TODO: Recursive traversal of statement AST
        // For each expression in statement:
        //   - Check if it's a method call
        //   - Look up method signature
        //   - Check parameter types
        false
    }
}
