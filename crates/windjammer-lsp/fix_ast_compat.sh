#!/bin/bash
# Automated script to fix remaining AST compatibility issues in windjammer-lsp

set -e

echo "🔧 Fixing LSP AST Compatibility Issues..."

# Fix analysis.rs - tokenize_with_locations
sed -i.bak 's/let tokens = lexer.tokenize();/let tokens = lexer.tokenize_with_locations();/g' src/analysis.rs

# Fix all remaining Item patterns
for file in src/completion.rs src/hover.rs src/semantic_tokens.rs src/server.rs src/symbol_table.rs; do
    if [ -f "$file" ]; then
        echo "Fixing $file..."
        # Item::Function
        sed -i.bak 's/Item::Function(func)/Item::Function { decl: func, location: _ }/g' "$file"
        sed -i.bak 's/Item::Function(_func_decl)/Item::Function { decl: _func_decl, location: _ }/g' "$file"
        sed -i.bak 's/Item::Function(_)/Item::Function { decl: _, location: _ }/g' "$file"
        
        # Item::Struct
        sed -i.bak 's/Item::Struct(struct_decl)/Item::Struct { decl: struct_decl, location: _ }/g' "$file"
        sed -i.bak 's/Item::Struct(s)/Item::Struct { decl: s, location: _ }/g' "$file"
        sed -i.bak 's/Item::Struct(_)/Item::Struct { decl: _, location: _ }/g' "$file"
        
        # Item::Enum
        sed -i.bak 's/Item::Enum(enum_decl)/Item::Enum { decl: enum_decl, location: _ }/g' "$file"
        sed -i.bak 's/Item::Enum(e)/Item::Enum { decl: e, location: _ }/g' "$file"
        sed -i.bak 's/Item::Enum(_)/Item::Enum { decl: _, location: _ }/g' "$file"
        
        # Item::Trait
        sed -i.bak 's/Item::Trait(trait_decl)/Item::Trait { decl: trait_decl, location: _ }/g' "$file"
        sed -i.bak 's/Item::Trait(t)/Item::Trait { decl: t, location: _ }/g' "$file"
        sed -i.bak 's/Item::Trait(_)/Item::Trait { decl: _, location: _ }/g' "$file"
        
        # Item::Impl
        sed -i.bak 's/Item::Impl(impl_block)/Item::Impl { block: impl_block, location: _ }/g' "$file"
        sed -i.bak 's/Item::Impl(_)/Item::Impl { block: _, location: _ }/g' "$file"
        
        # Clean up backup files
        rm -f "$file.bak"
    fi
done

echo "✅ AST compatibility fixes applied!"
echo "🔨 Building to verify..."

cargo build --quiet 2>&1 | head -20

echo ""
echo "✨ Fix script complete!"

