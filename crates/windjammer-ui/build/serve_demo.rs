



use std::http::*;

use std::fs::*;

use std::mime::*;


#[inline]
fn handle_request(req: &Request) -> ServerResponse {
    let path = req.path();
    println!("[REQUEST] {} {}", req.method(), path);
    let file_path = {
        if path == "/" {
            "examples/counter_demo.html"
        } else {
            path.trim_start_matches("/")
        }
    };
    match fs::read_to_string(file_path) {
        Ok(content) => {
            let mime_type = mime::from_path(file_path);
            println!("  ✅ Serving: {} ({})", file_path, mime_type);
            ServerResponse::ok(content).with_header("Content-Type", mime_type).with_header("Access-Control-Allow-Origin", "*").with_header("Cache-Control", "no-cache");
        },
        Err(_e) => {
            println!("  ❌ File not found: {}", file_path);
            let error_html = format!("<html><body><h1>404 Not Found</h1><p>{}</p></body></html>", file_path);
            ServerResponse::with_status(404, error_html).with_header("Content-Type", mime::TEXT_HTML);
        },
    }
}

#[tokio::main]
async fn main() {
    println!("");
    println!("🎨 Windjammer UI Demo Server");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("");
    println!("📂 Serving from: crates/windjammer-ui/");
    println!("🌐 Open in browser:");
    println!("");
    println!("   http://localhost:8000/examples/counter_demo.html");
    println!("");
    println!("✨ Features:");
    println!("   • Interactive counter with reactive state");
    println!("   • WASM-powered UI framework");
    println!("   • Proper MIME types via std::mime");
    println!("   • Served by Windjammer HTTP server!");
    println!("");
    println!("Press Ctrl+C to stop");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("");
    let router = Router::new().any("/*", handle_request);
    match http::serve("0.0.0.0:8000", router).await {
        Ok(x) => println!("Server stopped"),
        Err(e) => println!("Server error: {}", e),
    }
}

