# HTTP Server Example

A RESTful API server built with Windjammer, demonstrating:

- Decorator-based routing (`@get`, `@post`, `@delete`)
- Async/await for handling concurrent requests
- JSON serialization/deserialization
- Error handling with `Result<T, E>`
- Middleware for authentication
- Performance timing with `@timing` decorator

## Running

```bash
wj build
cd output
cargo run
```

## Testing

```bash
# List users
curl http://localhost:3000/users -H "X-API-Key: secret-key"

# Get specific user
curl http://localhost:3000/users/1 -H "X-API-Key: secret-key"

# Create user
curl -X POST http://localhost:3000/users \
  -H "X-API-Key: secret-key" \
  -H "Content-Type: application/json" \
  -d '{"name":"Charlie","email":"charlie@example.com"}'

# Delete user
curl -X DELETE http://localhost:3000/users/1 -H "X-API-Key: secret-key"
```

## Features Demonstrated

### Decorators
- `@get("/path")` - HTTP GET route
- `@post("/path")` - HTTP POST route
- `@delete("/path")` - HTTP DELETE route
- `@timing` - Automatic performance logging
- `@middleware` - Request/response middleware

### Async/Await
All handlers are async, allowing efficient concurrent request handling.

### Ownership Inference
Notice how we don't explicitly specify `&` or `&mut` in most places - the compiler infers it!

### Error Handling
Clean error propagation with `Result<T, E>` and the `?` operator.

