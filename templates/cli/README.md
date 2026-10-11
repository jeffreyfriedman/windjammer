# {{PROJECT_NAME}}

A command-line application built with Windjammer.

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

### Run with arguments

<!-- ignore: template for a new project: commands run inside a generated project with cargo and network access -->
```bash,ignore
wj run src/main.wj -- YourName
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

## Adding Dependencies

Use the `wj add` command to add dependencies:

<!-- ignore: template for a new project: commands run inside a generated project with cargo and network access -->
```bash,ignore
wj add reqwest      # HTTP client
wj add serde --features derive  # Serialization
```

## Learn More

- [Windjammer Documentation](https://github.com/windjammer-lang/windjammer)
- [Standard Library Guide](https://github.com/windjammer-lang/windjammer/blob/main/docs/GUIDE.md)
- [Examples](https://github.com/windjammer-lang/windjammer/tree/main/examples)

## License

MIT OR Apache-2.0
