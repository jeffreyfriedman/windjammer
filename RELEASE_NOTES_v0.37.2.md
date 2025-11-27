# Windjammer v0.37.2 Release Notes

## 🐛 Critical Bug Fixes

This patch release fixes several critical compiler bugs discovered during `windjammer-ui` v0.2.0 development (dogfooding effort).

### Compiler Fixes

1. **String Literal `.to_string()` Bug** (#[issue])
   - **Problem**: String literals were incorrectly getting `.to_string()` appended when passed to `push_str()`, causing type mismatches (`expected &str, found String`)
   - **Fix**: Removed incorrect `.to_string()` conversion in codegen for string literals passed to `push_str()`
   - **Impact**: All string operations now generate correct Rust code

2. **Tuple Pattern Matching with String Literals** (#[issue])
   - **Problem**: When matching on tuples containing string literals, the compiler incorrectly added `.as_str()` to the entire tuple expression
   - **Fix**: Added `is_tuple_match` check to prevent `.as_str()` from being applied to tuple values
   - **Impact**: Tuple pattern matching now works correctly with string literals

3. **AUTO-CLONE String Literal Variables** (#[issue])
   - **Problem**: The AUTO-CLONE analyzer was adding unnecessary `.clone()` calls to `&str` variables bound to string literals from match/if expressions, triggering `noop_method_call` clippy warnings
   - **Fix**: Implemented `string_literal_vars` tracking in `AutoCloneAnalysis` to identify variables bound to string literals and skip `.clone()` for them
   - **Impact**: Generated code is cleaner and passes clippy without warnings

4. **External Crate Naming** (#[issue])
   - **Problem**: Generated `Cargo.toml` files used underscores for external crate names (e.g., `windjammer_ui`) instead of hyphens (e.g., `windjammer-ui`), causing dependency resolution failures
   - **Fix**: Added `.replace('_', "-")` when extracting external crate names for `Cargo.toml` generation
   - **Impact**: External crate dependencies now resolve correctly from crates.io

## 🧹 Maintenance

- Removed 38 obsolete session/status markdown files from repository
- Added `.pr-comments/` to `.gitignore` for PR comment drafts
- Updated `README.md` version references

## 📦 Version Updates

- `windjammer`: 0.37.1 → 0.37.2
- `windjammer-lsp`: 0.36.1 → 0.37.2
- `windjammer-mcp`: 0.36.1 → 0.37.2
- `windjammer-runtime`: 0.36.1 → 0.37.2

## 🎯 Dogfooding Impact

These fixes were discovered and validated through the development of `windjammer-ui` v0.2.0, where **10 new UI components** (Chip, Form, FormField, Loading, Modal, Rating, Stack, Stepper, Table, Timeline) were written in pure Windjammer. This dogfooding effort proved that Windjammer is ready for real-world library development.

## 📝 Upgrade Notes

**Breaking Changes**: None

**Recommended Actions**:
1. Update to `windjammer = "0.37.2"` in your `Cargo.toml`
2. Run `cargo update` to get the latest version
3. Rebuild your projects - previously failing code should now compile correctly

## 🔗 Related PRs

- Windjammer PR: [feature/tuple-support](https://github.com/jeffreyfriedman/windjammer/pull/XX)
- Windjammer-UI PR: [feature/v0.2.0-improvements](https://github.com/jeffreyfriedman/windjammer-ui/pull/XX)

---

**Full Changelog**: https://github.com/jeffreyfriedman/windjammer/compare/v0.37.1...v0.37.2

