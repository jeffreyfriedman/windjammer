# Standard library: no WebSocket server

`std::net::WebSocket` has `connect` (a client). There is no way to accept WebSocket connections from Windjammer code
(`std::http` serves plain HTTP only), so a chat or push server cannot be written as a socket program. The chat core in
`windjammer-ecosystem/apps/wj-chat` is transport-independent for this reason and ships a scripted stdin/stdout adapter.

Wanted: a server-side API with the same shape as `std::http::server_serve` (`on_open(conn)`, `on_text(conn, text)`,
`on_close(conn)`, `send(conn, text)`), backed by the runtime's existing async stack.
