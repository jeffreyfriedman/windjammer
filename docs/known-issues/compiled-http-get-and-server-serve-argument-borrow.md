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
