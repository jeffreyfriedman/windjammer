# Contributing to Windjammer

Thank you for your interest in contributing to Windjammer! We're building a language that makes Rust's power accessible to everyone.

## 🌟 Our Vision

**The 80/20 Language**: 80% of Rust's power with 20% of its complexity.

We're creating a Go-like language that transpiles to Rust, combining:
- Go's simplicity and ergonomics
- Rust's safety and performance
- Modern language features (string interpolation, pipe operators, smart derives)

## 🚀 Quick Start

### Prerequisites
- Rust 1.70+ (for building the compiler)
- Basic familiarity with either Go or Rust

### Building
```bash
git clone https://github.com/yourusername/windjammer.git
cd windjammer
cargo build
cargo test
```

### Running
```bash
cargo run -- build --path examples/hello_world/main.wj
```

## 📝 How to Contribute

### Reporting Bugs
1. Check if the issue already exists
2. Include:
   - Windjammer code that triggers the bug
   - Expected behavior
   - Actual behavior (error message, generated Rust code)
   - Your OS and Rust version

### Suggesting Features
Before implementing, please:
1. Open an issue to discuss the feature
2. Explain how it fits the 80/20 philosophy
3. Provide examples showing the ergonomic improvement

We prefer features that:
- ✅ Reduce boilerplate by 80%+
- ✅ Are intuitive and consistent
- ✅ Cover common use cases
- ❌ Add complexity for edge cases
- ❌ Provide multiple ways to do the same thing

### Pull Requests

#### Before You Start
1. Comment on the issue you're working on
2. Fork the repository
3. Create a feature branch: `git checkout -b feature/my-feature`

#### Development Guidelines

**Code Style:**
- Follow Rust conventions (rustfmt, clippy)
- Add comments for complex logic
- Use descriptive variable names

**Testing:**
- Add tests for every feature in `tests/compiler_tests.rs`
- Add test fixtures in `tests/fixtures/`
- Ensure all tests pass: `cargo test`
- Test with real examples when possible

**Documentation:**
- Update `GUIDE.md` with usage examples
- Update `PROGRESS.md` to mark features as completed
- Add inline comments for non-obvious code
- Update README.md if the feature is user-facing

#### Commit Messages
Use clear, descriptive commit messages:
```
Add ternary operator support

- Implement parsing for condition ? true : false
- Add code generation to Rust if-else
- Disambiguate from TryOp (?) using lookahead
- Add comprehensive tests

Closes #42
```

#### Pull Request Process
1. **Create PR** with a clear description
2. **Link related issues** (e.g., "Closes #42")
3. **Pass CI checks** (all tests must pass)
4. **Respond to feedback** - we'll review within a few days
5. **Squash commits** if requested
6. **Celebrate** when merged! 🎉

## 🔍 Areas We Need Help With

### High Priority
- [ ] Error mapping (Rust errors → Windjammer source lines)
- [ ] Performance benchmarks
- [ ] Standard library modules (http, json, fs)
- [ ] LSP improvements (autocomplete, go-to-definition)

### Good First Issues
- [ ] Add more test cases for existing features
- [ ] Improve error messages
- [ ] Add examples to GUIDE.md
- [ ] Write doctests for stdlib

### Future Features
- [ ] Trait bound inference
- [ ] Package manager
- [ ] Doctest support
- [ ] WASM support improvements

Check our [Issues](https://github.com/yourusername/windjammer/issues) for specific tasks tagged `good-first-issue` or `help-wanted`.

## 🏗️ Architecture

Quick overview (see [ARCHITECTURE.md](ARCHITECTURE.md) for details):

```
.wj file → Lexer → Parser → Analyzer → Code Generator → .rs file
              ↓        ↓         ↓            ↓
           Tokens    AST    Ownership    Rust code
                              hints
```

**Key files:**
- `src/lexer.rs` - Tokenization
- `src/parser.rs` - AST construction
- `src/analyzer.rs` - Ownership inference
- `src/codegen.rs` - Rust code generation
- `src/main.rs` - CLI and build pipeline

## 📚 Resources

- **[GUIDE.md](GUIDE.md)** - Language tutorial
- **[COMPARISON.md](COMPARISON.md)** - Windjammer vs Rust vs Go
- **[SYNTAX_PROPOSALS.md](SYNTAX_PROPOSALS.md)** - Proposed features
- **[ERROR_MAPPING_DESIGN.md](ERROR_MAPPING_DESIGN.md)** - Error mapping design
- **[ROADMAP.md](ROADMAP.md)** - Long-term plans

## 💬 Community

- **GitHub Issues** - Bug reports, feature requests
- **GitHub Discussions** - Questions, ideas, show & tell
- **Discord** (coming soon) - Real-time chat

## ⚖️ Licensing

Windjammer is dual-licensed under MIT OR Apache-2.0.

### Contribution License
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in Windjammer by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

By submitting a pull request, you agree that your contributions will be licensed under both:
- MIT License ([LICENSE-MIT](LICENSE-MIT))
- Apache License 2.0 ([LICENSE-APACHE](LICENSE-APACHE))

This is the same licensing model as Rust itself.

## ✅ Code of Conduct

Be kind, respectful, and constructive:
- ✅ Welcome newcomers
- ✅ Be patient with questions
- ✅ Provide constructive feedback
- ✅ Focus on the code, not the person
- ❌ No harassment, discrimination, or toxicity

We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## 🎯 Design Philosophy

When in doubt, ask:

1. **Does it reduce boilerplate?** (80/20 rule)
2. **Is it consistent?** (one obvious way to do it)
3. **Is it safe?** (compile-time guarantees)
4. **Is it fast?** (should match Rust performance)
5. **Is it simple?** (understandable in 5 minutes)

If you can answer "yes" to all five, it's a good fit for Windjammer!

## 🙏 Thank You

Every contribution matters:
- Reporting a bug
- Fixing a typo
- Adding tests
- Improving documentation
- Implementing features

You're helping make Rust accessible to everyone. **Thank you!** 🚀

---

Questions? Open an issue or discussion. We're here to help!

