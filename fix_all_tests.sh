#!/bin/bash
set -e

echo "🔧 Fixing all test compilation errors..."

# Files with test errors
FILES=(
    "src/auto_clone.rs"
    "src/optimizer/phase12_dead_code_elimination.rs"
    "src/optimizer/phase13_loop_optimization.rs"
    "src/optimizer/phase15_simd_vectorization.rs"
)

for file in "${FILES[@]}"; do
    echo "Fixing $file..."
    
    # Fix Item::Function patterns
    sed -i '' -e 's/Item::Function(FunctionDecl {/Item::Function { decl: FunctionDecl {/g' "$file"
    sed -i '' -e 's/Item::Function(func)/Item::Function { decl: func, location: None }/g' "$file"
    
    # Fix Expression::Literal patterns
    sed -i '' -e 's/Expression::Literal(Literal::/Expression::Literal { value: Literal::/g' "$file"
    sed -i '' -e 's/Expression::Identifier("\([^"]*\)".to_string())/Expression::Identifier { name: "\1".to_string(), location: None }/g' "$file"
    
    # Fix Statement::Return patterns
    sed -i '' -e 's/Statement::Return(Some(/Statement::Return { value: Some(/g' "$file"
    sed -i '' -e 's/Statement::Return(None)/Statement::Return { value: None, location: None }/g' "$file"
    
    # Fix Statement::Expression patterns  
    sed -i '' -e 's/Statement::Expression(Expression::/Statement::Expression { expr: Expression::/g' "$file"
    
    # Add missing fields to FunctionDecl in tests
    sed -i '' -e 's/visibility: Visibility::Private,$/visibility: Visibility::Private,\n                type_params: vec![],\n                where_clause: None,\n                parent_type: None,/g' "$file"
    
    # Add location: None to Statement constructors
    sed -i '' -e 's/Statement::For {$/Statement::For {\n                    location: None,/g' "$file"
    
    # Add location to Expression::Call
    sed -i '' -e 's/Expression::Call {$/Expression::Call {\n                        location: None,/g' "$file"
done

echo "✅ Test fixes applied!"
