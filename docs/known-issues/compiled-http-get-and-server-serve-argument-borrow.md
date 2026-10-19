# Compiled: `http.get(url)` gets an owned `String`, `server_serve(addr, ..)` gets a borrowed one

```windjammer
use std::http

struct Request {
    url: string,
}

fn fetch(req: Request) -> int {
    match http.get(req.url) {
        Ok(response) => response.status_code(),
        Err(_) => 0,
    }
}
```

`wj build` emits `http::get(req.url.clone())` against the runtime `pub fn get(url: &str)` (`E0308: expected &str, found
String`). The mirror image: `server_serve(bind_addr, port, handler)` with `bind_addr` a local `string` is emitted as
`server_serve(&bind_addr, ..)` against `address: String` (`E0308: expected String, found &String`). Both follow the
runtime signature, which is not consulted for the argument's mode.

Status: fixed. Two causes. (1) A call through an imported runtime module (`http.get(req.url)`) took its argument mode from the WJ stub (`url: string`, owned), so an owned `req` produced `req.url.clone()`; the module call now takes the runtime function's signature when the runtime borrows the parameter (`method_call_expression_generation/arguments.rs`). (2) The multipass bare-pass demotion treated the empty `std/http.wj` stub body as a borrowing callee when a module passed a bare local (`server_serve(bind_addr, ..)`), emitting `&bind_addr` against `address: String`; a standard-library function's contract is the runtime's, so the demotion skips it (`multipass_bare_pass_demotion::bare_pass_hint_should_skip`). Gates: `ecosystem_findings2::http_module_call_in_a_project_module_borrows_the_field`, `server_serve_in_a_project_module_passes_the_owned_address`, `http_get_and_server_serve_follow_runtime_argument_modes`.
