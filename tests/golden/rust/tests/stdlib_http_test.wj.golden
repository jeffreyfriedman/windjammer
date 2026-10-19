use windjammer_runtime::test::*;

use windjammer_runtime::http;
use windjammer_runtime::http::HttpMethod;
use windjammer_runtime::http::Response;
use windjammer_runtime::http::Request;
use windjammer_runtime::http::ServerResponse;
use windjammer_runtime::http::Router;
use windjammer_runtime::http::ServerRequest;
use windjammer_runtime::http::Server;
#[test]

fn test_http_ok_response() {
    let response = http::ServerResponse::ok(String::from("Hello, World!"));
    assert!(response.status == 200_u16);
    assert!(response.body == "Hello, World!");
}

#[test]

fn test_http_not_found_response() {
    let not_found = http::ServerResponse::not_found();
    assert!(not_found.status == 404_u16);
}

#[test]

fn test_http_custom_response() {
    let mut custom = http::ServerResponse::ok(String::from("Custom"));
    custom.status = 201_u16;
    assert!(custom.status == 201_u16);
}

