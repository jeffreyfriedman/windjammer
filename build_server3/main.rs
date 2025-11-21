use windjammer_runtime::http;


#[tokio::main]
async fn main() {
    println!("=== Simple HTTP Server Demo ===");
    println!();
    println!("Starting server on http://0.0.0.0:8080...");
    println!("Visit http://localhost:8080 in your browser");
    println!();
    http::serve_fn("0.0.0.0:8080", move |req| {
        ServerResponse::ok("Hello from Windjammer!".to_string())
    }).await;
    println!();
    println!("✨ HTTP Server in ~3 lines of code!");
    println!("   ✅ No axum:: or rocket:: in your code");
    println!("   ✅ Clean Windjammer API");
    println!("   ✅ Perfect for quick prototypes")
}

