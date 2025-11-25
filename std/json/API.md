# std.json - JSON Serialization

Simple, ergonomic JSON parsing and serialization wrapping `serde_json`.

## API

### Parsing

```windjammer
use std::json

// Parse JSON string to value
fn parse(text: string) -> Result<Value, Error>

// Parse to specific type (with @auto(Deserialize))
fn parse_to<T>(text: string) -> Result<T, Error>
```

### Serialization

```windjammer
// Convert value to JSON string
fn stringify<T>(value: &T) -> Result<string, Error>

// Pretty print JSON
fn stringify_pretty<T>(value: &T) -> Result<string, Error>
```

### Value Type

```windjammer
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(string),
    Array(Vec<Value>),
    Object(HashMap<string, Value>),
}

impl Value {
    // Type checks
    fn is_null(&self) -> bool
    fn is_bool(&self) -> bool
    fn is_number(&self) -> bool
    fn is_string(&self) -> bool
    fn is_array(&self) -> bool
    fn is_object(&self) -> bool
    
    // Conversions
    fn as_bool(&self) -> Option<bool>
    fn as_i64(&self) -> Option<i64>
    fn as_f64(&self) -> Option<f64>
    fn as_str(&self) -> Option<&str>
    fn as_array(&self) -> Option<&Vec<Value>>
    fn as_object(&self) -> Option<&HashMap<string, Value>>
    
    // Access by key/index
    fn get(&self, key: string) -> Option<&Value>
    fn get_index(&self, index: int) -> Option<&Value>
}
```

## Example Usage

### Simple Parsing

```windjammer
use std::json

fn parse_config() -> Result<(), Error> {
    let text = "{\"name\": \"Alice\", \"age\": 30}"
    let value = json::parse(text)?
    
    if let Some(name) = value.get("name").as_str() {
        println!("Name: ${name}")
    }
    
    Ok(())
}
```

### Parsing to Struct

```windjammer
use std::json

@auto(Debug, Deserialize, Serialize)
struct User {
    name: string,
    age: int,
    email: string,
}

fn load_users() -> Result<Vec<User>, Error> {
    let text = fs::read_to_string("users.json")?
    let users = json::parse_to<Vec<User>>(text)?
    Ok(users)
}
```

### Creating JSON

```windjammer
use std::json

@auto(Serialize)
struct Config {
    host: string,
    port: int,
    debug: bool,
}

fn save_config() -> Result<(), Error> {
    let config = Config {
        host: "localhost",
        port: 8080,
        debug: true,
    }
    
    let json_text = json::stringify_pretty(&config)?
    fs::write("config.json", json_text)?
    
    Ok(())
}
```

### Working with Dynamic JSON

```windjammer
use std::json

fn process_api_response(response: string) -> Result<(), Error> {
    let data = json::parse(response)?
    
    // Access nested data
    if let Some(users) = data.get("users").as_array() {
        for user in users {
            let name = user.get("name").as_str().unwrap_or("Unknown")
            let age = user.get("age").as_i64().unwrap_or(0)
            println!("${name} is ${age} years old")
        }
    }
    
    Ok(())
}
```

### Using with HTTP

```windjammer
use std::http
use std::json

@auto(Deserialize)
struct ApiResponse {
    status: string,
    data: Vec<Item>,
}

fn fetch_items() -> Result<Vec<Item>, Error> {
    let response = http::get("https://api.example.com/items")?
    let api_response = json::parse_to<ApiResponse>(response.body)?
    Ok(api_response.data)
}
```

### Pipe Operator Integration

```windjammer
fn load_and_process() -> Result<(), Error> {
    fs::read_to_string("data.json")?
        |> json::parse?
        |> extract_relevant_data
        |> process_data
        |> save_results
}
```

### Builder Pattern for JSON

```windjammer
fn create_json_object() -> Value {
    json.object([
        ("name", json.string("Alice")),
        ("age", json.number(30)),
        ("active", json.bool(true)),
        ("tags", json.array([
            json.string("developer"),
            json.string("rust"),
        ])),
    ])
}
```

### Error Handling

```windjammer
match json::parse(text) {
    Ok(value) => {
        println!("Parsed successfully")
    }
    Err(e) => {
        println!("Parse error: ${e}")
        // Handle malformed JSON
    }
}
```

## Automatic Serialization

Use `@auto(Serialize, Deserialize)` on structs:

```windjammer
@auto(Debug, Serialize, Deserialize)
struct Product {
    id: int,
    name: string,
    price: f64,
    in_stock: bool,
    tags: Vec<string>,
}

// Automatically works with json::parse_to and json::stringify
let product = Product {
    id: 1,
    name: "Widget",
    price: 29.99,
    in_stock: true,
    tags: vec!["gadget", "useful"],
}

let json_text = json::stringify(&product)?
// Output: {"id":1,"name":"Widget","price":29.99,"in_stock":true,"tags":["gadget","useful"]}
```

## Features

✅ **Simple API** - Parse and stringify in one call  
✅ **Type-Safe** - Use structs with @auto derive  
✅ **Dynamic Access** - Work with unknown JSON structures  
✅ **Pretty Printing** - Readable JSON output  
✅ **Error Handling** - Clear error messages  
✅ **Zero Copy** - Efficient parsing  
✅ **Pipe Operator Support** - Chainable operations  

---

**Status**: API Design Complete  
**Implementation**: Pending  
**Rust Deps**: `serde`, `serde_json`

