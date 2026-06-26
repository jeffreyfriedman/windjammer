use windjammer_runtime::http::*;
fn main() {
    println!("=== Simple HTTP Server Demo ===");
    println!("");
    println!("Starting server on http://0.0.0.0:8080...");
    println!("Visit http://localhost:8080 in your browser");
    println!("");
    let server = Server::new("0.0.0.0".to_string(), 8080);
    match server.serve(|req| handle(&req)) {
        Ok(_) => println!("Server stopped"),
        Err(err) => println!("Server error: {}", err),
    }
    println!("");
    println!("HTTP server demo complete");
}

#[inline]
fn handle(_req: &ServerRequest) -> ServerResponse {
    ServerResponse::ok("Hello from Windjammer!")
}

