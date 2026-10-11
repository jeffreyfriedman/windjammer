# {{PROJECT_NAME}}

A web application built with Windjammer.

## Getting Started

### Run the application

<!-- ignore: template for a new project: commands run inside a generated project with cargo and network access -->
```bash,ignore
wj run src/main.wj
```

### Build for release

<!-- ignore: template for a new project: commands run inside a generated project with cargo and network access -->
```bash,ignore
wj build --release
```

## Project Structure

```
{{PROJECT_NAME}}/
├── src/
│   └── main.wj     # Main application code
├── wj.toml         # Project configuration
├── .gitignore      # Git ignore rules
└── README.md       # This file
```

## Features

- HTTP client using `std.http`
- JSON serialization using `std.json`
- Async/await support
- Type-safe API models

## Adding Dependencies

Use the `wj add` command to add dependencies:

<!-- ignore: template for a new project: commands run inside a generated project with cargo and network access -->
```bash,ignore
wj add serde --features derive  # Serialization
wj add tokio --features full    # Async runtime
```

## Example API Usage

```windjammer
use std::http
use std::json

@async
fn main() {
    let response = http::get("https://api.example.com/data").await?
    let data = response.json::<MyType>().await?
    println("Received: ${data:?}")
}
```

## Learn More

- [Windjammer Documentation](https://github.com/windjammer-lang/windjammer)
- [Standard Library Guide](https://github.com/windjammer-lang/windjammer/blob/main/docs/GUIDE.md)
- [HTTP Module Documentation](https://github.com/windjammer-lang/windjammer/blob/main/docs/GUIDE.md#stdhttp)

## License

MIT OR Apache-2.0

