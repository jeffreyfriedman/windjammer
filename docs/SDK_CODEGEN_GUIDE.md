# Windjammer SDK Code Generation Guide

## Overview

Windjammer uses an **Interface Definition Language (IDL)** approach to generate SDKs for 12 programming languages from a single source of truth. This ensures API consistency, reduces maintenance burden, and enables rapid multi-language support.

## Architecture

```
┌─────────────────────┐
│  windjammer_api.json│  ← Single source of truth
└──────────┬──────────┘
           │
           ▼
    ┌──────────────┐
    │ SDK Generator│  ← Code generation tool
    └──────┬───────┘
           │
           ├─────────┬─────────┬─────────┬─────────┬─────────┐
           ▼         ▼         ▼         ▼         ▼         ▼
        Rust     Python    TypeScript   C#       C++       Go
        Java     Kotlin      Lua      Swift     Ruby

```

## Components

### 1. API Definition (`api/windjammer_api.json`)

JSON file defining the complete Windjammer API:
- **Structs**: Data structures (Vec2, Vec3, Time)
- **Classes**: OOP classes (App, Camera2D, Sprite)
- **Enums**: Enumerated types
- **Functions**: Free functions
- **Constants**: Global constants
- **Modules**: API organization

### 2. SDK Generator (`tools/sdk-generator`)

CLI tool that generates SDK code:

```bash
# Generate all SDKs
cargo run --bin wj-sdk-gen -- --all

# Generate specific languages
cargo run --bin wj-sdk-gen -- --languages rust python typescript

# Custom API definition
cargo run --bin wj-sdk-gen -- --api custom_api.json --languages go
```

### 3. Docker Test Infrastructure (`docker/`)

Docker containers for testing each SDK:
- `Dockerfile.python` - Python 3.11 + pytest
- `Dockerfile.javascript` - Node 20 + Jest
- `Dockerfile.go` - Go 1.21 + testing
- `Dockerfile.java` - Maven + JUnit
- `Dockerfile.csharp` - .NET 8 + xUnit
- `Dockerfile.cpp` - GCC 13 + CMake
- `Dockerfile.kotlin` - Gradle + JUnit

### 4. Test Script (`scripts/test-all-sdks.sh`)

Automated testing for all SDKs:

```bash
./scripts/test-all-sdks.sh
```

## Workflow

### 1. Define API

Edit `api/windjammer_api.json`:

```json
{
  "structs": [
    {
      "name": "Vec2",
      "fields": [
        {
          "name": "x",
          "field_type": { "Primitive": "Float32" },
          "doc": "X coordinate",
          "public": true
        }
      ],
      "doc": "2D vector",
      "generics": []
    }
  ]
}
```

### 2. Generate SDKs

```bash
cd tools/sdk-generator
cargo run -- --all
```

### 3. Test SDKs

```bash
# Test all SDKs
./scripts/test-all-sdks.sh

# Test specific SDK
docker-compose -f docker/docker-compose.test.yml run test-python
```

### 4. Publish SDKs

Each SDK has its own publishing process:
- **Rust**: `cargo publish`
- **Python**: `twine upload`
- **JavaScript/TypeScript**: `npm publish`
- **C#**: `dotnet nuget push`
- **Java**: `mvn deploy`
- **Go**: Git tag + Go modules
- **Kotlin**: `gradle publish`

## Type Mapping

The code generator automatically maps IDL types to language-specific types:

| IDL Type | Rust | Python | TypeScript | C# | C++ | Go | Java |
|----------|------|--------|------------|----|----|-----|------|
| `Bool` | `bool` | `bool` | `boolean` | `bool` | `bool` | `bool` | `boolean` |
| `Int32` | `i32` | `int` | `number` | `int` | `int32_t` | `int32` | `int` |
| `Float32` | `f32` | `float` | `number` | `float` | `float` | `float32` | `float` |
| `String` | `String` | `str` | `string` | `string` | `std::string` | `string` | `String` |
| `Array<T>` | `Vec<T>` | `list` | `T[]` | `List<T>` | `std::vector<T>` | `[]T` | `ArrayList<T>` |
| `Optional<T>` | `Option<T>` | `T \| None` | `T \| null` | `T?` | `std::optional<T>` | `*T` | `Optional<T>` |

## Benefits

### 1. **Single Source of Truth**
- API defined once in `windjammer_api.json`
- No drift between language implementations
- Easy to add new APIs

### 2. **Consistency**
- All SDKs have identical APIs
- Same documentation across languages
- Predictable behavior

### 3. **Maintainability**
- Update API once, regenerate all SDKs
- Automated testing catches issues early
- Reduced maintenance burden

### 4. **Rapid Language Support**
- Add new language by implementing code generator
- Existing API definitions work immediately
- Docker testing ensures quality

## Adding a New Language

1. **Add language to `sdk_codegen.rs`**:
   ```rust
   pub enum Language {
       // ... existing languages
       NewLanguage,
   }
   ```

2. **Implement type mapping**:
   ```rust
   Language::NewLanguage => {
       self.type_map.insert("bool".to_string(), "Bool".to_string());
       // ... more mappings
   }
   ```

3. **Implement code generation**:
   ```rust
   fn generate_struct(&self, struct_def: &StructDef) -> Result<String, CodeGenError> {
       match self.language {
           Language::NewLanguage => {
               // Generate struct code
           }
       }
   }
   ```

4. **Create Dockerfile**:
   ```dockerfile
   FROM newlang:latest
   WORKDIR /sdk
   COPY sdks/newlang/ ./
   CMD ["newlang", "test"]
   ```

5. **Add to test script**:
   ```bash
   SDKS=("python" "javascript" "newlang")
   ```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Test SDKs

on: [push, pull_request]

jobs:
  test-sdks:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        sdk: [python, javascript, go, java, csharp, cpp, kotlin]
    steps:
      - uses: actions/checkout@v3
      - name: Test ${{ matrix.sdk }} SDK
        run: |
          docker-compose -f docker/docker-compose.test.yml run test-${{ matrix.sdk }}
```

## Best Practices

1. **Always regenerate after API changes**
2. **Run tests before committing**
3. **Document new APIs in IDL**
4. **Version API definitions**
5. **Keep language-specific code minimal**
6. **Use Docker for reproducible testing**

## Troubleshooting

### SDK Generation Fails
- Check `windjammer_api.json` syntax
- Ensure all types are defined
- Verify code generator is up to date

### Tests Fail
- Check Docker container logs
- Verify SDK dependencies are installed
- Ensure API definition matches implementation

### Type Mapping Issues
- Check `init_type_map()` in `sdk_codegen.rs`
- Verify custom types are defined
- Ensure language supports the type

## Future Enhancements

- [ ] Automatic documentation generation
- [ ] API versioning and migration
- [ ] Performance benchmarking
- [ ] Cross-language integration tests
- [ ] IDE plugin generation
- [ ] API diff tool for breaking changes
- [ ] Automated publishing pipeline

## Resources

- [IDL Specification](sdk_idl.rs)
- [Code Generator](sdk_codegen.rs)
- [Docker Compose](../docker/docker-compose.test.yml)
- [Test Script](../scripts/test-all-sdks.sh)

