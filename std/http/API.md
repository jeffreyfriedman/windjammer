# std.http - HTTP Client

Ergonomic HTTP client wrapping `reqwest` with Windjammer conveniences.

## API

### Simple Requests

```windjammer
use std.http

// GET request
fn get(url: string) -> Result<Response, Error>

// POST request with body
fn post(url: string, body: string) -> Result<Response, Error>

// PUT request
fn put(url: string, body: string) -> Result<Response, Error>

// DELETE request
fn delete(url: string) -> Result<Response, Error>
```

### Request Builder (Advanced)

```windjammer
// Create a custom request
fn request() -> RequestBuilder

impl RequestBuilder {
    fn url(&mut self, url: string) -> &mut Self
    fn method(&mut self, method: string) -> &mut Self
    fn header(&mut self, key: string, value: string) -> &mut Self
    fn json<T>(&mut self, data: &T) -> &mut Self
    fn body(&mut self, body: string) -> &mut Self
    fn timeout(&mut self, seconds: int) -> &mut Self
    fn send(&self) -> Result<Response, Error>
}
```

### Response

```windjammer
struct Response {
    status: int,
    headers: HashMap<string, string>,
    body: string,
}

impl Response {
    fn text(&self) -> string
    fn json<T>(&self) -> Result<T, Error>
    fn bytes(&self) -> Vec<u8>
    fn is_success(&self) -> bool
    fn is_error(&self) -> bool
}
```

## Example Usage

### Simple GET Request

```windjammer
use std.http
use std.json

fn fetch_users() -> Result<Vec<User>, Error> {
    let response = http.get("https://api.example.com/users")?
    
    if response.is_success() {
        let users = json.parse(response.body)?
        Ok(users)
    } else {
        Err(Error::new("Failed to fetch users"))
    }
}
```

### POST with JSON

```windjammer
use std.http
use std.json

fn create_user(name: string, email: string) -> Result<User, Error> {
    let user_data = json.object([
        ("name", name),
        ("email", email),
    ])
    
    let response = http.post(
        "https://api.example.com/users",
        json.stringify(user_data)
    )?
    
    let user = json.parse(response.body)?
    Ok(user)
}
```

### Advanced Request with Headers

```windjammer
use std.http

fn authenticated_request(token: string) -> Result<Response, Error> {
    http.request()
        .url("https://api.example.com/data")
        .method("GET")
        .header("Authorization", "Bearer ${token}")
        .header("Content-Type", "application/json")
        .timeout(30)
        .send()
}
```

### Using Pipe Operator

```windjammer
fn fetch_and_process() -> Result<Vec<ProcessedData>, Error> {
    "https://api.example.com/data"
        |> http.get?
        |> Response.json?
        |> process_data
        |> Ok
}
```

### Error Handling

```windjammer
match http.get("https://api.example.com/data") {
    Ok(response) => {
        println!("Status: ${response.status}")
        println!("Body: ${response.body}")
    }
    Err(e) => {
        println!("Request failed: ${e}")
    }
}
```

## Types

```windjammer
enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

struct Headers {
    items: HashMap<string, string>,
}

impl Headers {
    fn get(&self, key: string) -> Option<string>
    fn set(&mut self, key: string, value: string)
    fn remove(&mut self, key: string)
}
```

## Configuration

```windjammer
// Set default timeout for all requests
http.set_default_timeout(60)

// Set default headers
http.set_default_header("User-Agent", "Windjammer/1.0")
```

## Features

✅ Automatic JSON serialization/deserialization  
✅ Timeout support  
✅ Custom headers  
✅ All HTTP methods  
✅ Error handling with `?` operator  
✅ Async support (via `async fn`)  

---

**Status**: API Design Complete  
**Implementation**: Pending  
**Rust Deps**: `reqwest` (with blocking + json features)

