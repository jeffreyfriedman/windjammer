# Windjammer Versioning Strategy

## Current Situation
- **Cargo.toml**: `version = "0.1.0"`
- **Docs**: References to "v0.3"
- **Git tags**: None yet
- **GitHub releases**: None yet

**Problem**: Inconsistent versioning!

---

## Proposed Versioning Plan

### Version 0.1.0 - Initial Release
**Tag**: `v0.1.0`  
**What**: Current `main` branch (initial commit)  
**When**: Now (retroactively tag initial commit)

**Features**:
- Core compiler pipeline (lexer, parser, analyzer, codegen)
- Basic language features (functions, structs, impl blocks, etc.)
- String interpolation
- Pipe operator
- @auto derive (explicit)
- Trait system
- 8/9 tests passing

---

### Version 0.2.0 - Assignment Statements & Full Test Coverage
**Tag**: `v0.2.0`  
**What**: This feature branch (`feature/expand-tests-and-examples`)  
**When**: After merging this PR

**New features**:
- ✅ Assignment statements (`x = x + 1`)
- ✅ Mutable borrow inference from assignments
- ✅ 9/9 tests passing (100% coverage)
- ✅ Ternary operator
- ✅ Smart @auto derive (zero-config)
- ✅ Better documentation

**Breaking changes**: None  
**Upgrade path**: Seamless

---

### Future Versions

#### Version 0.3.0 - Character Literals & Working Examples
**Planned features**:
- Character literal support (`'a'`, `'x'`)
- All 4 examples working
- 5+ additional simple examples
- Better error messages with line numbers

#### Version 0.4.0 - Local Variable Tracking
**Planned features**:
- Local variable ownership tracking
- Closure capture analysis
- Move semantics for locals
- Compound assignments (`+=`, `-=`)

#### Version 0.5.0 - Error Mapping
**Planned features**:
- Rust error → Windjammer source mapping
- Source maps
- Better error messages

#### Version 0.6.0 - Standard Library & Tooling
**Planned features**:
- Core stdlib modules (http, json, fs)
- LSP improvements (autocomplete, go-to-definition)
- Performance benchmarks
- Comprehensive examples
- VS Code extension complete

#### Future: Version 1.0.0 - Production Ready
**Not planned yet** - Will stay in 0.x until:
- Battle-tested in real projects
- Community feedback incorporated
- API stability proven
- Performance validated
- All major features complete and stable

**0.6.0 will be the "feature complete" milestone before 1.0 consideration**

---

## Semantic Versioning

Following [SemVer 2.0.0](https://semver.org/):

**MAJOR.MINOR.PATCH**

- **MAJOR** (0 → 1): Language is production-ready, API stable
- **MINOR** (0.x → 0.y): New features, backward compatible
- **PATCH** (0.x.y → 0.x.z): Bug fixes, no new features

**Pre-1.0 Note**: During 0.x, minor versions may include breaking changes.

---

## How to Tag Versions

### Tag Initial Release (v0.1.0)
```bash
# Switch to main
git checkout main

# Tag the initial commit
git tag -a v0.1.0 -m "Release v0.1.0 - Initial compiler implementation

Features:
- Core compiler pipeline
- Basic language features
- String interpolation, pipe operator
- Trait system
- 8/9 tests passing"

# Push tag to GitHub
git push origin v0.1.0
```

### Tag This Branch (v0.2.0)
```bash
# After merging feature branch to main
git checkout main
git pull origin main

# Update Cargo.toml version
# (Change version = "0.1.0" to version = "0.2.0")

# Commit version bump
git add Cargo.toml CHANGELOG.md
git commit -m "Bump version to 0.2.0"
git push origin main

# Create annotated tag
git tag -a v0.2.0 -m "Release v0.2.0 - Assignment statements and full test coverage

New features:
- Assignment statements (x = x + 1)
- Mutable borrow inference from assignments  
- Ternary operator (condition ? true : false)
- Smart @auto derive (zero-config trait inference)
- 9/9 tests passing (100% coverage)

Breaking changes: None
Upgrade: Seamless"

# Push tag
git push origin v0.2.0
```

### Create GitHub Release
After pushing tags, go to GitHub:
1. Navigate to: https://github.com/jeffreyfriedman/windjammer/releases
2. Click "Draft a new release"
3. Choose tag (v0.2.0)
4. Release title: "Windjammer v0.2.0 - Assignment Statements"
5. Copy release notes from tag message
6. Attach binaries (optional)
7. Click "Publish release"

---

## Version Checklist

Before tagging a version:
- [ ] Update `Cargo.toml` version number
- [ ] Update `CHANGELOG.md` with release notes
- [ ] All tests passing
- [ ] Documentation updated
- [ ] README.md reflects current state
- [ ] Breaking changes documented
- [ ] Migration guide (if needed)

---

## CHANGELOG Format

Keep `CHANGELOG.md` updated following [Keep a Changelog](https://keepachangelog.com):

```markdown
## [0.2.0] - 2025-10-03

### Added
- Assignment statements support
- Mutable borrow inference from assignments
- Ternary operator

### Fixed
- Analyzer warning (dead code)

### Changed
- Updated README with all features
```

---

## Git Tag Commands Reference

```bash
# List all tags
git tag

# Show tag details
git show v0.2.0

# Delete local tag (if mistake)
git tag -d v0.2.0

# Delete remote tag (if mistake)
git push origin :refs/tags/v0.2.0

# Push all tags
git push origin --tags

# Fetch tags from remote
git fetch --tags
```

---

## Recommendation

**For this PR**, I suggest:

1. **Now**: Retroactively tag main as `v0.1.0` (initial release)
2. **After merge**: 
   - Update Cargo.toml to `0.2.0`
   - Update CHANGELOG.md
   - Tag as `v0.2.0`
   - Create GitHub release

This gives us:
- Clean version history
- Semantic versioning from the start
- Easy rollback if needed
- Professional release process

**Want me to prepare the version bump commits?**

