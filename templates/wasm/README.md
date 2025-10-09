# {{PROJECT_NAME}}

A WebAssembly application built with Windjammer.

## Getting Started

### Build for WASM

```bash
wj build --target wasm
```

This generates a WASM package in the `build_output/pkg` directory.

### Run in Browser

Start a local web server:

```bash
cd www
python3 -m http.server 8000
```

Then open http://localhost:8000 in your browser.

## Project Structure

```
{{PROJECT_NAME}}/
├── src/
│   └── main.wj     # WASM module code
├── www/
│   └── index.html  # Web page
├── wj.toml         # Project configuration
├── .gitignore      # Git ignore rules
└── README.md       # This file
```

## Exported Functions

The following functions are exported to JavaScript:

- `greet(name: string) -> string` - Returns a greeting
- `add(a: i32, b: i32) -> i32` - Adds two numbers
- `fibonacci(n: i32) -> i32` - Calculates Fibonacci number

## JavaScript Usage

```javascript
import init, { greet, add, fibonacci } from './pkg/{{PROJECT_NAME}}.js';

await init();

console.log(greet('World'));        // "Hello from Windjammer, World!"
console.log(add(2, 3));             // 5
console.log(fibonacci(10));         // 55
```

## Notes

- Use `i32` instead of `int` for WASM compatibility with JavaScript
- Use `f64` for floating-point numbers
- Functions must be marked with `@export` to be callable from JavaScript

## Learn More

- [Windjammer Documentation](https://github.com/windjammer-lang/windjammer)
- [WASM Guide](https://github.com/windjammer-lang/windjammer/blob/main/docs/GUIDE.md#wasm)
- [Examples](https://github.com/windjammer-lang/windjammer/tree/main/examples/wasm_hello)

## License

MIT OR Apache-2.0

