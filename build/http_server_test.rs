use windjammer_runtime::http;


#[inline]
fn handle_index(req: &http::Request) -> http::ServerResponse {
    return http::ServerResponse::ok("Hello from Windjammer HTTP Server!".to_string());
}

#[inline]
fn handle_json(req: &http::Request) -> http::ServerResponse {
    let data = "{ "message": "Hello, JSON!" }";
    return http::ServerResponse::ok(data.to_string()).with_header("Content-Type".to_string(), "application/json".to_string());
}

fn main() {
    println!("Starting Windjammer HTTP server on http://127.0.0.1:8080");
    let router = http::Router::new().get("/", handle_index).get("/json", handle_json);
    let result = http::serve("127.0.0.1:8080", router);
    if result.is_ok() {
        println!("Server stopped")
    } else {
        println!("Server error")
    }
}

