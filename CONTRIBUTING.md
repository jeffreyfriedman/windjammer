# Contributing to Windjammer

Thank you for your interest in contributing to Windjammer! This document provides guidelines and instructions for contributing.

## 🌟 Ways to Contribute

There are many ways to contribute to Windjammer:

- 🐛 **Report bugs** - Help us identify and fix issues
- ✨ **Suggest features** - Share ideas for new functionality
- 📚 **Improve documentation** - Help others learn Windjammer
- 💻 **Submit code** - Fix bugs or implement features
- 🧪 **Write tests** - Improve code coverage and reliability
- 💬 **Help others** - Answer questions in discussions
- 🎨 **Share examples** - Create tutorials and sample projects

## 📋 Before You Start

1. **Check existing issues** - Search for similar issues or feature requests
2. **Read the docs** - Familiarize yourself with Windjammer's philosophy and design
3. **Join discussions** - Engage with the community to discuss your ideas
4. **Start small** - Begin with small contributions to understand the codebase

## 🐛 Reporting Bugs

When reporting a bug, please include:

- **Clear title** - Summarize the issue concisely
- **Description** - Explain what happened and what you expected
- **Reproduction steps** - Provide a minimal example that reproduces the bug
- **Environment** - Include Windjammer version, OS, and any relevant details
- **Error messages** - Include full error output and stack traces

Use the [Bug Report template](.github/ISSUE_TEMPLATE/bug_report.yml) when creating an issue.

## ✨ Suggesting Features

When suggesting a feature:

- **Explain the problem** - What need does this feature address?
- **Propose a solution** - How should it work?
- **Consider alternatives** - What other approaches did you consider?
- **Show examples** - Provide code examples of how you'd use it
- **Align with philosophy** - Does it fit "80% of Rust's power, 20% of complexity"?

Use the [Feature Request template](.github/ISSUE_TEMPLATE/feature_request.yml) when creating an issue.

## 💻 Development Setup

### Prerequisites

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- Git
- A code editor (VS Code with rust-analyzer recommended)

### Getting Started

1. **Fork the repository**
   ```bash
   # Click "Fork" on GitHub, then clone your fork
   git clone https://github.com/YOUR_USERNAME/windjammer.git
   cd windjammer
   ```

2. **Create a branch**
   ```bash
   git checkout -b feature/your-feature-name
   # or
   git checkout -b fix/your-bug-fix
   ```

3. **Build the project**
   ```bash
   cargo build
   ```

4. **Run tests**
   ```bash
   cargo test
   ```

5. **Install pre-commit hooks**
   ```bash
   git config core.hooksPath .git/hooks
   ln -sf ../../.githooks/pre-commit .git/hooks/pre-commit
   ```

## 🧪 Testing

- **Run all tests**: `cargo test`
- **Run specific test**: `cargo test test_name`
- **Run with output**: `cargo test -- --nocapture`
- **Run benchmarks**: `cargo bench`

Always add tests for new features and bug fixes.

## 📝 Code Style

- **Format code**: `cargo fmt --all`
- **Lint code**: `cargo clippy -- -D warnings`
- **Follow conventions**: Match the existing code style
- **Add comments**: Explain complex logic and design decisions
- **Write docs**: Add documentation for public APIs

## 🔄 Pull Request Process

1. **Update your branch**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

2. **Make your changes**
   - Write clear, focused commits
   - Follow conventional commit format: `type: description`
   - Types: `feat`, `fix`, `docs`, `refactor`, `test`, `chore`

3. **Test thoroughly**
   - Run all tests: `cargo test`
   - Run clippy: `cargo clippy`
   - Format code: `cargo fmt --all`

4. **Update documentation**
   - Update README.md if needed
   - Update CHANGELOG.md for user-facing changes
   - Add/update inline documentation

5. **Push and create PR**
   ```bash
   git push origin your-branch-name
   ```
   - Go to GitHub and create a pull request
   - Fill out the PR template completely
   - Link related issues

6. **Respond to feedback**
   - Address review comments promptly
   - Make requested changes
   - Ask questions if anything is unclear

## 📦 Project Structure

```
windjammer/
├── src/               # Core compiler source
│   ├── parser/        # Windjammer language parser
│   ├── analyzer/      # Semantic analysis and type checking
│   ├── codegen/       # Code generation (Rust, JS, WASM)
│   └── cli/           # Command-line interface
├── std/               # Standard library (.wj files)
├── crates/            # Sub-crates
│   ├── windjammer-lsp/      # Language Server Protocol
│   ├── windjammer-mcp/      # Model Context Protocol
│   └── windjammer-runtime/  # Runtime support
├── examples/          # Example projects
├── docs/              # Documentation
└── tests/             # Integration tests
```

## 🎯 Windjammer Philosophy

When contributing, keep in mind Windjammer's core philosophy:

- **80% of Rust's power, 20% of the complexity**
- **Zero backend leakage** - Developers write pure Windjammer
- **Inferred ownership** - No explicit `mut`, `&`, or lifetimes
- **Multi-target** - Compile to Rust, JavaScript, and WebAssembly
- **Pragmatic** - Favor practical solutions over theoretical purity

## 🏷️ Commit Message Guidelines

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): description

[optional body]

[optional footer]
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation changes
- `refactor`: Code refactoring
- `test`: Test additions or updates
- `chore`: Maintenance tasks
- `perf`: Performance improvements

**Examples:**
```
feat(parser): Add support for async/await syntax
fix(codegen): Correct ownership inference for builder patterns
docs(std): Update http module API documentation
```

## 📜 License

By contributing to Windjammer, you agree that your contributions will be licensed under the same license as the project (MIT OR Apache-2.0).

## 🤝 Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## 💬 Getting Help

- **Discussions**: Ask questions in [GitHub Discussions](https://github.com/jeffreyfriedman/windjammer/discussions)
- **Issues**: Report bugs or request features
- **Documentation**: Read the [official docs](docs/)

## 🙏 Thank You!

Your contributions make Windjammer better for everyone. We appreciate your time and effort!

---

**Questions?** Feel free to ask in [Discussions](https://github.com/jeffreyfriedman/windjammer/discussions) or open an issue.
