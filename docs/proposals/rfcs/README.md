# Windjammer RFCs (Request for Comments)

This directory contains formal proposals for significant changes to the Windjammer language, standard library, and tooling.

## Active RFCs

### Security Framework

- **[WJ-SEC-01: Inferred Effect Capabilities](./WJ-SEC-01-effect-capabilities.md)** - Compile-time permission system to prevent supply chain attacks
- **[WJ-SEC-02: Taint Tracking](./WJ-SEC-02-taint-tracking.md)** - Type-level injection prevention (SQL injection, XSS, command injection)

### Syntax Improvements

- **[WJ-SYN-01: Pipe Operator](./WJ-SYN-01-pipe-operator.md)** - Optional ergonomic improvement for chaining functions

## RFC Process

### Lifecycle

1. **Draft** - Initial proposal, open for discussion
2. **Review** - Community feedback and iteration
3. **Accepted** - Approved for implementation
4. **Implemented** - Feature is live in specified version
5. **Rejected** - Not proceeding with this proposal

### Status Legend

- 🟡 **Draft** - Under initial development
- 🔵 **Review** - Open for feedback
- 🟢 **Accepted** - Approved, ready for implementation
- ⚫ **Implemented** - Feature is live
- 🔴 **Rejected** - Not proceeding

## Current Status

| RFC | Status | Target Version | Priority |
|-----|--------|----------------|----------|
| WJ-SEC-01 | 🟡 Draft | v0.50 | High |
| WJ-SEC-02 | 🟡 Draft | v0.55 | High |
| WJ-SYN-01 | 🟡 Draft | v0.60+ | Low |

## Contributing

To propose a new RFC:

1. Create a new file `WJ-[CATEGORY]-[NUMBER]-[short-name].md`
2. Follow the existing RFC format (see WJ-SEC-01 as template)
3. Submit for review
4. Address feedback and iterate

**Categories:**
- **SEC** - Security features
- **SYN** - Syntax improvements
- **STD** - Standard library additions
- **TOOL** - Tooling improvements
- **LANG** - Core language features

## Philosophy Alignment

All RFCs must align with the [Windjammer Philosophy](../../../.cursor/rules/windjammer-development.mdc):

- **No Rust Leakage** - Examples must be idiomatic Windjammer (no `&`, `&mut`, `.as_str()`)
- **Compiler Does the Hard Work** - Automatic inference over manual annotations
- **80/20 Rule** - 80% of power with 20% of complexity
- **Backend-Agnostic** - Features work across Rust, Go, JavaScript, Interpreter backends
- **TDD + Dogfooding** - Tests first, real-world validation

## See Also

- [Syntax Improvement Proposals](../syntax.md) - Historical syntax discussions
- [WJSL RFC](../../WJSL_RFC.md) - Shader language proposal
- [Development Rules](../../../.cursor/rules/windjammer-development.mdc) - Core development principles
