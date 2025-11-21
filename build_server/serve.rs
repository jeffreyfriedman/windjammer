


use windjammer_runtime::http;

use windjammer_runtime::fs;


#[inline]
fn serve_static_file(path: &String) -> ServerResponse {
    let file_path = {
        if path == "/" {
            "index.html".to_string()
        } else {
            path.trim_start_matches('/').to_string()
        }
    };
    match fs::read_to_string(file_path.clone()) {
        Ok(content) => {
            let content_type = {
                if file_path.ends_with(".html") {
                    "text/html; charset=utf-8"
                } else {
                    if file_path.ends_with(".js") {
                        "application/javascript; charset=utf-8"
                    } else {
                        if file_path.ends_with(".wasm") {
                            "application/wasm"
                        } else {
                            if file_path.ends_with(".css") {
                                "text/css; charset=utf-8"
                            } else {
                                if file_path.ends_with(".json") {
                                    "application/json; charset=utf-8"
                                } else {
                                    "text/plain; charset=utf-8"
                                }
                            }
                        }
                    }
                }
            };
            ServerResponse::ok(content).with_header("Content-Type", content_type).with_header("Cache-Control", "no-cache")
        },
        Err(e) => {
            println!("File not found: {} ({})", file_path, e);
            ServerResponse::not_found()
        },
    }
}

#[inline]
fn handle_request(req: &Request) -> ServerResponse {
    let path = req.path();
    println!("📄 {} {}", req.method(), path);
    serve_static_file(&path)
}

#[tokio::main]
async fn main() {
    println!("🚀 Windjammer WASM Editor Server");
    println!("   (Dogfooding our own HTTP server!)");
    println!();
    println!("📁 Serving from: ./");
    println!("🌐 URL: http://localhost:8080");
    println!();
    println!("Files available:");
    println!("  /                    → index.html");
    println!("  /pkg/*.js            → JavaScript bindings");
    println!("  /pkg/*.wasm          → WASM binary");
    println!();
    println!("Starting server...");
    println!();
    let router = Router::new().get("/*", handle_request);
    match http::serve(&"0.0.0.0:8080", router).await {
        Ok(_) => println!("✓ Server stopped"),
        Err(e) => println!("✗ Server error: {}", e),
    }
}

