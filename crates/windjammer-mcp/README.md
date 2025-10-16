# Windjammer MCP Server

**Version**: 0.31.0  
**Status**: Beta  
**Protocol**: Model Context Protocol (MCP) 2025-06-18

---

## Overview

The Windjammer Model Context Protocol (MCP) server enables AI assistants (Claude, ChatGPT, and others) to deeply understand, analyze, and generate Windjammer code. It provides a standardized interface for AI-powered development tools to interact with Windjammer codebases.

**Key Features**:
- 🤖 **AI-Native**: Built specifically for AI assistant integration
- 🎯 **Rich Tools**: Parse, analyze, generate, and refactor Windjammer code
- 🔍 **Deep Understanding**: Leverages same Salsa database as LSP for consistency
- ⚡ **Fast**: Incremental computation with sub-millisecond cached queries
- 🛡️ **Secure**: Input validation, sandboxing, and resource limits
- 📚 **Comprehensive**: 6+ tools covering code understanding, generation, and errors

---

## Quick Start

### Installation

```bash
cargo install windjammer-mcp
```

Or build from source:

```bash
cd crates/windjammer-mcp
cargo build --release
```

### Running the Server

```bash
# Run with stdio transport (default)
windjammer-mcp

# Or explicitly
windjammer-mcp stdio

# Show server info
windjammer-mcp info
```

### Integration with Claude Desktop

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json` on macOS):

```json
{
  "mcpServers": {
    "windjammer": {
      "command": "/path/to/windjammer-mcp",
      "args": ["stdio"]
    }
  }
}
```

### Integration with Other AI Assistants

The MCP server uses JSON-RPC 2.0 over stdio, making it compatible with any AI assistant that supports MCP:

```python
# Python example
import subprocess
import json

# Start the server
server = subprocess.Popen(
    ["windjammer-mcp", "stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    text=True
)

# Send initialize request
request = {
    "jsonrpc": "2.0",
    "id": 1,
    "method": "initialize",
    "params": {
        "protocolVersion": "2024-11-05",
        "capabilities": {},
        "clientInfo": {"name": "my-client", "version": "1.0.0"}
    }
}

server.stdin.write(json.dumps(request) + "\n")
server.stdin.flush()

# Read response
response = json.loads(server.stdout.readline())
print(response)
```

---

## Available Tools

### 1. `parse_code`
Parse Windjammer code and return AST structure.

**Input**:
```json
{
  "code": "fn main() { println!(\"Hello\") }",
  "include_diagnostics": true
}
```

**Output**:
```json
{
  "success": true,
  "ast": { ... },
  "diagnostics": []
}
```

**Use Cases**:
- Validate syntax before code generation
- Extract structure from existing code
- Detect parse errors

---

### 2. `analyze_types`
Perform type inference and analysis.

**Input**:
```json
{
  "code": "let x = 42; let y = x + 1",
  "cursor_position": {"line": 1, "column": 9}
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

**Use Cases**:
- Understand inferred types in code
- Verify type correctness
- Get type information at specific positions

---

### 3. `generate_code`
Generate Windjammer code from natural language.

**Input**:
```json
{
  "description": "Create a function that filters even numbers from a vector"
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

**Use Cases**:
- Bootstrap new functions from descriptions
- Generate boilerplate code
- Learn idiomatic Windjammer patterns

---

### 4. `explain_error`
Explain compiler errors in plain English.

**Input**:
```json
{
  "error": "error[E0308]: mismatched types\\n  expected `i64`, found `&str`",
  "code_context": "let x: int = \"hello\""
}
```

**Output**:
```json
{
  "success": true,
  "explanation": "You're trying to assign a string (\\\"hello\\\") to a variable declared as an integer (int). Windjammer requires types to match exactly.",
  "suggestion": "Change the type to `string` or change the value to a number like `42`.",
  "corrected_code": "let x: string = \"hello\"  // or: let x: int = 42"
}
```

**Use Cases**:
- Help beginners understand errors
- Provide quick fixes for common mistakes
- Learn from error messages

---

### 5. `get_definition`
Find the definition of a symbol.

**Input**:
```json
{
  "symbol": "add",
  "file": "src/main.wj",
  "position": {"line": 5, "column": 10}
}
```

**Output**:
```json
{
  "success": true,
  "definition": {
    "file": "src/lib.wj",
    "range": {
      "start": {"line": 10, "column": 0},
      "end": {"line": 12, "column": 1}
    },
    "signature": "fn add(a: int, b: int) -> int"
  }
}
```

**Use Cases**:
- Navigate to function/type definitions
- Understand symbol origins
- Explore codebase structure

---

### 6. `search_workspace`
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

**Use Cases**:
- Find examples of specific patterns
- Locate functions by signature
- Understand how features are used

---

## Architecture

### Shared Database with LSP

The MCP server shares the same Salsa-powered incremental computation database with the Windjammer LSP. This ensures:

- ✅ **Consistency**: Same parsing and analysis results
- ✅ **Performance**: Cached computations benefit both LSP and MCP
- ✅ **Accuracy**: No divergence between IDE and AI tools

```
┌──────────────┐     ┌──────────────┐
│  LSP Client  │     │ MCP Client   │
│  (VSCode)    │     │  (Claude)    │
└──────┬───────┘     └──────┬───────┘
       │                    │
       ▼                    ▼
┌──────────────────────────────────┐
│   Shared Salsa Database          │
│   - Incremental parsing          │
│   - Type inference cache         │
│   - Symbol resolution            │
└──────────────────────────────────┘
```

### Security

- **Input Validation**: All inputs validated against JSON schemas
- **Resource Limits**: Code size limited to 1MB, timeouts on operations
- **Sandboxing**: Analysis runs in isolated database instances
- **No File System Access**: By default (without explicit permission)

---

## Examples

### Example 1: Parse and Validate Code

```python
import json

request = {
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
        "name": "parse_code",
        "arguments": {
            "code": "fn add(a: int, b: int) -> int { a + b }",
            "include_diagnostics": true
        }
    }
}

# Send to server...
# Response will include AST and any parse errors
```

### Example 2: Generate Code from Description

```python
request = {
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
        "name": "generate_code",
        "arguments": {
            "description": "HTTP server that responds 'Hello World' on GET /"
        }
    }
}

# Response includes generated Windjammer code
```

### Example 3: Explain an Error

```python
request = {
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
        "name": "explain_error",
        "arguments": {
            "error": "cannot find value `foo` in this scope",
            "code_context": "println!(foo)"
        }
    }
}

# Response includes plain English explanation and suggestions
```

---

## Development

### Building

```bash
cargo build --release
```

### Testing

```bash
cargo test
```

### Benchmarking

```bash
cargo bench
```

---

## Protocol Reference

### Initialization

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2024-11-05",
    "capabilities": {},
    "clientInfo": {
      "name": "client-name",
      "version": "1.0.0"
    }
  }
}
```

### Tool List

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/list"
}
```

### Tool Call

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "tool_name",
    "arguments": { ... }
  }
}
```

### Shutdown

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "shutdown"
}
```

---

## Roadmap

### v0.31.0 (Current)
- [x] Core MCP server with stdio transport
- [x] Basic tools: parse, analyze, generate, explain, search
- [x] Integration with LSP database
- [x] Unit tests

### v0.32.0 (Future)
- [ ] Streamable HTTP transport ([MCP 2025-06-18 spec](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports))
- [ ] Session management with Mcp-Session-Id header
- [ ] Resumable streams with Last-Event-ID
- [ ] OAuth 2.0 authentication
- [ ] Advanced refactoring tools
- [ ] Workspace-wide operations
- [ ] Performance benchmarks

### v0.33.0 (Future)
- [ ] Custom tool plugins
- [ ] Multi-language support
- [ ] Production deployment guides
- [ ] AI agent integration examples

---

## Contributing

We welcome contributions! See [../../CONTRIBUTING.md](../../CONTRIBUTING.md) for guidelines.

---

## License

Windjammer MCP is dual-licensed under either:

- **MIT License** ([../../LICENSE-MIT](../../LICENSE-MIT))
- **Apache License, Version 2.0** ([../../LICENSE-APACHE](../../LICENSE-APACHE))

at your option.

---

## Links

- **Main Repository**: https://github.com/jeffreyfriedman/windjammer
- **Windjammer Website**: https://windjammer.dev (coming soon)
- **MCP Specification**: https://modelcontextprotocol.io/
- **Issue Tracker**: https://github.com/jeffreyfriedman/windjammer/issues

---

**Questions?** Open an issue or check the [Windjammer Guide](../../docs/GUIDE.md).


