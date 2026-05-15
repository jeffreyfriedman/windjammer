#!/usr/bin/env python3
import re
import sys

def remove_ternary_arms(filename):
    with open(filename, 'r') as f:
        content = f.read()
    
    # Pattern to match Expression::Ternary match arms
    # This is a simple heuristic - match from "Expression::Ternary" to the closing brace
    pattern = r'Expression::Ternary\s*\{[^}]*\}\s*=>\s*(?:Expression::Ternary\s*\{[^}]*\}|[^,}]*),?\s*'
    
    # Remove all occurrences
    new_content = re.sub(pattern, '', content)
    
    with open(filename, 'w') as f:
        f.write(new_content)
    
    print(f"Processed {filename}")

if __name__ == "__main__":
    files = [
        "src/analyzer.rs",
        "src/inference.rs",
        "src/optimizer/phase12_dead_code_elimination.rs",
        "src/optimizer/phase13_loop_optimization.rs",
        "src/component/analyzer.rs"
    ]
    
    for f in files:
        remove_ternary_arms(f)


