# {{PROJECT_NAME}}

A library built with Windjammer.

## Getting Started

### Run tests

```bash
wj test
```

### Build the library

```bash
wj build --release
```

## Project Structure

```
{{PROJECT_NAME}}/
├── src/
│   └── lib.wj      # Library code
├── wj.toml         # Project configuration
├── .gitignore      # Git ignore rules
└── README.md       # This file
```

## Usage

Add this library to another project's `wj.toml`:

```toml
[dependencies]
{{PROJECT_NAME}} = { path = "../{{PROJECT_NAME}}" }
```

Then use it:

```windjammer
use {{PROJECT_NAME}}

fn main() {
    let greeting = {{PROJECT_NAME}}.hello("World")
    println!("{}", greeting)
    
    let sum = {{PROJECT_NAME}}.add(2, 3)
    println!("Sum: {}", sum)
}
```

## API Documentation

### Functions

- `hello(name: string) -> string` - Returns a greeting message
- `add(a: int, b: int) -> int` - Adds two integers
- `multiply(a: int, b: int) -> int` - Multiplies two integers

### Types

- `Point { x: float, y: float }` - A 2D point
  - `Point.new(x, y)` - Create a new point
  - `Point.distance(other)` - Calculate distance to another point

## Testing

Run the test suite:

```bash
wj test
```

Run specific tests:

```bash
wj test --filter test_hello
```

## Learn More

- [Windjammer Documentation](https://github.com/windjammer-lang/windjammer)
- [Standard Library Guide](https://github.com/windjammer-lang/windjammer/blob/main/docs/GUIDE.md)
- [Testing Guide](https://github.com/windjammer-lang/windjammer/blob/main/docs/GUIDE.md#testing)

## License

MIT OR Apache-2.0

