


use std::http::*;

use std::crypto::*;


#[inline]
fn generate_request_id() -> String {
    let random_bytes = crypto::random_bytes(16);
    let id = crypto::base64_encode(&random_bytes);
    format!("req_{}", id)
}

#[inline]
fn add_request_id_header(req: &Request) -> Request {
    let request_id = match http.get_header(req, "X-Request-ID") {
        Some(id) => id,
        None => generate_request_id(),
    };
    http::set_context(req, "request_id", request_id);
    req
}

#[inline]
fn add_request_id_to_response(res: &Response, request_id: &String) -> Response {
    http::set_header(res, "X-Request-ID", request_id)
}

