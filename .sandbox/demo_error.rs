// Demo of the new error format
fn main() {
    println!("=== Windjammer Error Message Demo ===\n");
    
    println!("error: Expected ']', got '}}'");
    println!("  --> test.wj:3:15");
    println!("   |");
    println!("  3 |     let x = [1, 2, 3");
    println!("   |               ^");
    println!("   = help: Add ']' before the newline");
    println!("   = suggestion: let x = [1, 2, 3]");
    
    println!("\n=== Compare to old format ===\n");
    println!("Parse error: Expected RBracket, got RBrace (at token position 18)");
    
    println!("\n✨ Much better!");
}
