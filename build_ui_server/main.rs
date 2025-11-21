


use std::fmt::Write;

use windjammer_runtime::http;

use windjammer_runtime::fs;


#[inline]
fn serve_file(path: &String) -> ServerResponse {
    let base_path = "crates/windjammer-ui/examples";
    let full_path = {
        let mut __s = String::with_capacity(64);
        write!(&mut __s, "{}/{}", base_path, path).unwrap();
        __s
    };
    match fs::read_to_string(full_path) {
        Ok(content) => {
            let content_type = {
                if path.ends_with(".html") {
                    "text/html"
                } else {
                    if path.ends_with(".js") {
                        "application/javascript"
                    } else {
                        if path.ends_with(".wasm") {
                            "application/wasm"
                        } else {
                            if path.ends_with(".css") {
                                "text/css"
                            } else {
                                "text/plain"
                            }
                        }
                    }
                }
            };
            ServerResponse::ok(content).with_header("Content-Type", content_type)
        },
        Err(_) => ServerResponse::not_found(),
    }
}

#[inline]
fn handle_index(req: &Request) -> ServerResponse {
    serve_file(&"showcase.html")
}

#[inline]
fn handle_static(req: &Request) -> ServerResponse {
    let path = req.path().trim_start_matches('/');
    serve_file(&path)
}

#[tokio::main]
async fn main() {
    println!("🚀 Windjammer UI Server");
    println!();
    println!("Serving from: crates/windjammer-ui/examples");
    println!();
    println!("Available pages:");
    println!("  http://localhost:8080/           - Full showcase");
    println!("  http://localhost:8080/showcase.html");
    println!("  http://localhost:8080/simple_counter.html");
    println!();
    println!("Starting server on http://0.0.0.0:8080...");
    println!();
    let router = Router::new().get("/", handle_index).get("/*", handle_static);
    match http.serve("0.0.0.0:8080", router).await {
        Ok(_) => println!("Server stopped"),
        Err(e) => println!("Server error: {}", e),
    }
}

