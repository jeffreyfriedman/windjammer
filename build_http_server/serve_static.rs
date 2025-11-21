


use windjammer_runtime::http;

use windjammer_runtime::fs;


#[inline]
fn handle_request(req: &Request) -> ServerResponse {
    let path = req.path();
    let file_path = {
        if path == "/" {
            "index.html"
        } else {
            path.trim_start_matches('/')
        }
    };
    println!("Serving: {}", file_path);
    match fs::read_to_string(file_path) {
        Ok(content) => {
            let content_type = {
                if file_path.ends_with(".html") {
                    "text/html"
                } else {
                    if file_path.ends_with(".js") {
                        "application/javascript"
                    } else {
                        if file_path.ends_with(".wasm") {
                            "application/wasm"
                        } else {
                            "text/plain"
                        }
                    }
                }
            };
            ServerResponse::ok(content).with_header("Content-Type", content_type)
        },
        Err(_) => ServerResponse::not_found(),
    }
}

#[tokio::main]
async fn main() {
    println!("Starting server on http://localhost:8080");
    let router = Router::new().get("/*", handle_request);
    http::serve(&"0.0.0.0:8080", router).await
}

