# Windjammer MCP Server Design

**Version**: 0.31.0  
**Status**: In Development  
**Target**: AI-First Development with Model Context Protocol

---

## Overview

The Windjammer Model Context Protocol (MCP) server enables AI assistants (Claude, ChatGPT, etc.) to understand, analyze, and generate Windjammer code through a standardized protocol.

**Key Goals**:
- 🤖 Enable AI agents to write idiomatic Windjammer code
- 🔍 Provide deep code understanding and context
- ⚡ Share infrastructure with existing LSP for consistency
- 🛡️ Secure, validated, and sandboxed operations
- 📚 Comprehensive tool library for common tasks

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      AI Assistant                            │
│              (Claude, ChatGPT, etc.)                         │
└───────────────────────┬─────────────────────────────────────┘
                        │ JSON-RPC over stdio/HTTP
                        ▼
┌─────────────────────────────────────────────────────────────┐
│                   MCP Server (Rust)                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Tool Registry                                       │   │
│  │  - parse_code      - generate_code                  │   │
│  │  - explain_error   - suggest_fix                    │   │
│  │  - refactor        - search_code                    │   │
│  └─────────────────────────────────────────────────────┘   │
│                          │                                   │
│                          ▼                                   │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Shared Salsa Database (with LSP)                   │   │
│  │  - Incremental parsing & analysis                   │   │
│  │  - Type inference cache                             │   │
│  │  - Symbol resolution                                │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

---

## MCP Protocol Implementation

### Transport Layers

1. **stdio** (Primary - v0.31.0) ✅
   - Process-based communication
   - Used by most AI assistants (Claude, etc.)
   - Simple, secure, no network exposure
   - Messages delimited by newlines
   - Full JSON-RPC 2.0 support

2. **Streamable HTTP** (v0.31.1) ✅
   - Modern replacement for deprecated HTTP+SSE
   - Follows MCP 2025-06-18 specification
   - Single POST endpoint for all requests
   - Session management with `Mcp-Session-Id` header
   - Session TTL and automatic cleanup
   - Concurrent session support
   - Secure by default (no network exposure without opt-in)
   
3. **OAuth 2.0 Authentication** (Future - v0.32.0)
   - Enterprise-grade authentication
   - Token-based access control
   - Integration with identity providers

### Message Format

All messages use JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "parse_code",
    "arguments": {
      "code": "fn main() { println!(\"Hello\") }"
    }
  }
}
```

---

## MCP Tools

### Code Understanding Tools

#### 1. `parse_code`
Parse Windjammer code and return AST structure.

**Input**:
```json
{
  "code": "fn add(a: int, b: int) -> int { a + b }",
  "include_diagnostics": true
}
```

**Output**:
```json
{
  "success": true,
  "ast": {
    "type": "Program",
    "items": [
      {
        "type": "Function",
        "name": "add",
        "parameters": [...],
        "return_type": "int",
        "body": [...]
      }
    ]
  },
  "diagnostics": []
}
```

#### 2. `analyze_types`
Perform type inference and analysis.

**Input**:
```json
{
  "code": "let x = 42; let y = x + 1",
  "cursor_position": { "line": 1, "column": 9 }
}
```

**Output**:
```json
{
  "success": true,
  "inferred_types": {
    "x": "i64",
    "y": "i64"
  },
  "type_at_cursor": "i64"
}
```

#### 3. `get_definition`
Find the definition of a symbol.

**Input**:
```json
{
  "file": "src/main.wj",
  "symbol": "add",
  "position": { "line": 5, "column": 10 }
}
```

**Output**:
```json
{
  "success": true,
  "definition": {
    "file": "src/lib.wj",
    "range": {
      "start": { "line": 10, "column": 0 },
      "end": { "line": 12, "column": 1 }
    },
    "signature": "fn add(a: int, b: int) -> int"
  }
}
```

---

### Code Generation Tools

#### 4. `generate_code`
Generate Windjammer code from natural language description.

**Input**:
```json
{
  "description": "Create a function that filters a vector of numbers, keeping only evens",
  "context": {
    "existing_functions": ["process_data", "validate_input"],
    "imports": ["std.collections"]
  }
}
```

**Output**:
```json
{
  "success": true,
  "code": "fn filter_evens(numbers: Vec<int>) -> Vec<int> {\n    numbers.iter().filter(|&n| n % 2 == 0).collect()\n}",
  "explanation": "This function uses Windjammer's iterator methods to filter even numbers."
}
```

#### 5. `complete_code`
Provide code completion suggestions.

**Input**:
```json
{
  "code": "fn process(data: Vec<",
  "cursor_position": { "line": 1, "column": 23 }
}
```

**Output**:
```json
{
  "success": true,
  "completions": [
    { "text": "int>", "kind": "type", "detail": "Vec<int>" },
    { "text": "string>", "kind": "type", "detail": "Vec<string>" },
    { "text": "T>", "kind": "type_parameter", "detail": "Generic type T" }
  ]
}
```

---

### Refactoring Tools

#### 6. `extract_function`
Extract selected code into a new function.

**Input**:
```json
{
  "file": "src/main.wj",
  "range": {
    "start": { "line": 10, "column": 4 },
    "end": { "line": 15, "column": 5 }
  },
  "new_function_name": "process_item"
}
```

**Output**:
```json
{
  "success": true,
  "changes": {
    "src/main.wj": {
      "edits": [
        {
          "range": { "start": {...}, "end": {...} },
          "new_text": "fn process_item(item: Item) -> Result<(), Error> {\n    // extracted code\n}"
        }
      ]
    }
  }
}
```

#### 7. `inline_variable`
Inline a variable's value at all usage sites.

**Input**:
```json
{
  "code": "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}",
  "position": { "line": 1, "column": 8 }
}
```

**Output**:
```json
{
  "success": true,
  "refactored_code": "fn main() {\n    println!(\"{}\", 42);\n}",
  "occurrences_replaced": 1,
  "variable_name": "x"
}
```

#### 8. `rename_symbol`
Rename a symbol with workspace-wide updates.

**Input**:
```json
{
  "code": "fn add(a: int, b: int) -> int { a + b }",
  "position": { "line": 0, "column": 7 },
  "new_name": "sum"
}
```

**Output**:
```json
{
  "success": true,
  "refactored_code": "fn sum(a: int, b: int) -> int { a + b }",
  "occurrences_renamed": 3,
  "old_name": "add",
  "files_affected": ["src/main.wj", "src/lib.wj"]
}
```

---

### Error Handling Tools

#### 9. `explain_error`
Explain a Windjammer error in plain English.

**Input**:
```json
{
  "error": "error[E0308]: mismatched types\n  expected `i64`, found `&str`",
  "code_context": "let x: int = \"hello\""
}
```

**Output**:
```json
{
  "success": true,
  "explanation": "You're trying to assign a string (\"hello\") to a variable declared as an integer (int). Windjammer requires types to match exactly.",
  "suggestion": "Change the type to `string` or change the value to a number like `42`.",
  "corrected_code": "let x: string = \"hello\"  // or: let x: int = 42"
}
```

#### 10. `suggest_fix`
Suggest automatic fixes for common errors.

---

### Workspace Tools

#### 11. `search_workspace`
Search for code patterns across the workspace.

**Input**:
```json
{
  "query": "functions that return Result<T, Error>",
  "file_pattern": "src/**/*.wj"
}
```

**Output**:
```json
{
  "success": true,
  "results": [
    {
      "file": "src/lib.wj",
      "matches": [
        {
          "line": 15,
          "signature": "fn read_file(path: string) -> Result<string, Error>",
          "context": "..."
        }
      ]
    }
  ]
}
```

#### 12. `get_file_context`
Get full context of a file (imports, types, functions).

#### 13. `list_symbols`
List all symbols in a file or workspace.

---

## Security & Validation

### Input Validation

All inputs are validated against JSON schemas:

```rust
#[derive(Deserialize, Validate)]
struct ParseCodeRequest {
    #[validate(length(min = 1, max = 1_000_000))]
    code: String,
    
    #[serde(default)]
    include_diagnostics: bool,
}
```

### Sandboxing

- All code analysis runs in isolated Salsa database instances
- No file system access without explicit permission
- Resource limits on parsing (time, memory)

### Error Handling

```rust
#[derive(Serialize)]
#[serde(tag = "error_type")]
enum McpError {
    ParseError { message: String, location: Option<Location> },
    ValidationError { field: String, message: String },
    InternalError { message: String },
    Timeout { duration_ms: u64 },
}
```

---

## Integration with LSP

### Shared Salsa Database

```rust
// crates/windjammer-lsp/src/database.rs
#[salsa::db(CompilerStorage)]
pub struct WindjammerDatabase {
    storage: salsa::Storage<Self>,
}

// crates/windjammer-mcp/src/server.rs
use windjammer_lsp::database::WindjammerDatabase;

pub struct McpServer {
    db: Arc<Mutex<WindjammerDatabase>>,  // Shared with LSP
    // ...
}
```

### Benefits

- ✅ Consistent parsing/analysis results
- ✅ No redundant computation
- ✅ Incremental updates benefit both
- ✅ Unified symbol resolution

---

## Performance

### Caching Strategy

```rust
// All queries are automatically memoized by Salsa
#[salsa::tracked]
fn parse_code(db: &dyn Db, source: SourceText) -> ParseResult {
    // Cached - only recomputes on source change
}
```

### Benchmarks (Target)

| Operation | Cold | Cached |
|-----------|------|--------|
| Parse 1000 lines | ~50ms | ~20ns |
| Type inference | ~100ms | ~50ns |
| Find references | ~200ms | ~100ns |
| Generate code | ~500ms | N/A |

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_parse_code_tool() {
    let server = McpServer::new();
    let request = ParseCodeRequest {
        code: "fn main() {}".to_string(),
        include_diagnostics: true,
    };
    
    let response = server.handle_parse_code(request).unwrap();
    assert!(response.success);
    assert_eq!(response.diagnostics.len(), 0);
}
```

### Integration Tests

- End-to-end tool invocation
- Multi-file workspace operations
- Error recovery scenarios

### AI Agent Tests

- Test with actual Claude/ChatGPT integrations
- Validate code generation quality
- Measure latency and success rates

---

## Deployment

### Local (stdio)

```bash
# AI assistant spawns process
windjammer-mcp --stdio
```

### Remote (HTTP + SSE)

```bash
# Server mode
windjammer-mcp --http --port 8080 --auth-token $TOKEN

# Client connects
curl -H "Authorization: Bearer $TOKEN" \
     http://localhost:8080/mcp \
     -d '{"jsonrpc":"2.0","method":"tools/list","id":1}'
```

---

## Roadmap

### v0.31.0 (Current)
- [x] Design document
- [ ] Core MCP server with stdio transport
- [ ] Basic tools: parse, analyze, generate
- [ ] Integration with LSP database
- [ ] Unit tests

### v0.32.0 (Future)
- [ ] Streamable HTTP transport (MCP 2025-06-18 spec)
- [ ] Session management with Mcp-Session-Id
- [ ] OAuth 2.0 authentication
- [ ] Advanced refactoring tools
- [ ] Workspace-wide operations
- [ ] AI agent benchmarks

### v0.33.0 (Future)
- [ ] Custom tool plugins
- [ ] Multi-language support (transpile to JS/Rust)
- [ ] Performance optimizations
- [ ] Production deployment guides

---

## References

- [Model Context Protocol Specification](https://modelcontextprotocol.io/)
- [Anthropic MCP SDK](https://github.com/anthropics/anthropic-sdk-python)
- [JSON-RPC 2.0](https://www.jsonrpc.org/specification)
- [Salsa Incremental Computation](https://github.com/salsa-rs/salsa)

