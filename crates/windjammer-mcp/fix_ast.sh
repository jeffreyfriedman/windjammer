#!/bin/bash
FILES=(
    "src/tools/refactor_extract_function.rs"
    "src/tools/refactor_inline_variable.rs"
)

for file in "${FILES[@]}"; do
    echo "Fixing $file..."
    # Statement patterns
    sed -i '' -e 's/Statement::Assignment { target, value }/Statement::Assignment { target, value, location: _ }/g' "$file"
    sed -i '' -e 's/Statement::Return(Some(expr))/Statement::Return { value: Some(expr), location: _ }/g' "$file"
    sed -i '' -e 's/Statement::Return(Some(_expr))/Statement::Return { value: Some(_expr), location: _ }/g' "$file"
    sed -i '' -e 's/Statement::Expression(expr)/Statement::Expression { expr, location: _ }/g' "$file"
    sed -i '' -e 's/Statement::Expression(_expr)/Statement::Expression { expr: _expr, location: _ }/g' "$file"
    sed -i '' -e 's/Statement::While { condition, body }/Statement::While { condition, body, location: _ }/g' "$file"
    
    # Expression patterns
    sed -i '' -e 's/Expression::Identifier(name)/Expression::Identifier { name, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::Identifier(_)/Expression::Identifier { name: _, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::Identifier("x".to_string())/Expression::Identifier { name: "x".to_string(), location: None }/g' "$file"
    sed -i '' -e 's/Expression::Identifier("foo".to_string())/Expression::Identifier { name: "foo".to_string(), location: None }/g' "$file"
    sed -i '' -e 's/Expression::Block(stmts)/Expression::Block { statements: stmts, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::Block(_)/Expression::Block { statements: _, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::Await(_)/Expression::Await { expr: _, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::ChannelRecv(_)/Expression::ChannelRecv { channel: _, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::Literal(_)/Expression::Literal { value: _, location: _ }/g' "$file"
    sed -i '' -e 's/Expression::Literal(Literal::Int(42))/Expression::Literal { value: Literal::Int(42), location: None }/g' "$file"
done

echo "Done!"
