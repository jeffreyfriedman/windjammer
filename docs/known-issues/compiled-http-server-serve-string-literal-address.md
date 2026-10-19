# Compiled: `server_serve("0.0.0.0", port, handler)` emits `&String::from("0.0.0.0")` for an owned `String` parameter

```windjammer
use std::http::*

fn main() {
    match server_serve("0.0.0.0", 8080, |req| ServerResponse::ok("hi")) {
        Ok(_) => {},
        Err(e) => eprintln("${e}"),
    }
}
```

The runtime signature is `server_serve(address: String, port: i64, handler)`; the emitted call passes
`&String::from("0.0.0.0")` (`E0308`, rustc: "consider removing the borrow"). Passing the literal through a helper
`fn bind_address() -> string { "0.0.0.0" }` avoids it, which is what `wj-notes-api` does.

Status: fixed. Single-file builds were already correct; the failure was project builds, where the empty `std/http.wj` stub body made analysis infer `Borrowed` for every parameter and that replaced the runtime's owned `address: String`. Std stub analysis no longer overrides runtime-scanned contracts. Gate: `ecosystem_findings_test::server_serve_string_literal_address_is_owned`.
