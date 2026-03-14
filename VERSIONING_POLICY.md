# Windjammer Versioning Policy

## Current Versions

### Windjammer Compiler: 0.46.0

**Components at 0.46.0:**
- `windjammer` (compiler)
- `windjammer-runtime` (standard library)
- `windjammer-lsp` (language server, if exists)
- All other compiler child crates

### Separate Projects (Independent Versioning)

**Windjammer Game Engine: 0.1.0**
- `windjammer-game` (game engine library)
- `windjammer-game-core` (core engine)
- `windjammer-runtime-host` (runtime)
- `wj-game` (build plugin)

**Breach Protocol: 0.1.0**
- `breach-protocol` (game binary)
- All breach-protocol crates

## Versioning Independence

**IMPORTANT:** These are SEPARATE projects:
1. **Windjammer** = Compiler (language tooling)
2. **Windjammer-Game** = Game engine (library using Windjammer)
3. **Breach Protocol** = Reference game (dogfooding project)

**Each has independent versioning!**

## Version Format

`MAJOR.MINOR.PATCH`

- **MAJOR:** Breaking changes
- **MINOR:** New features, non-breaking
- **PATCH:** Bug fixes only

## Naming Convention

| Component | Crate Name | Binary Name |
|-----------|------------|-------------|
| Compiler | `windjammer` | `wj` |
| Runtime | `windjammer-runtime` | N/A (library) |
| Game Engine | `windjammer-app` (windjammer-game-core) | N/A (library) |
| Runtime Host | `breach-protocol-host` | `breach-protocol-host` |

**Historical note:** Compiler was incorrectly named `windjammer-app` (fixed 2026-03-14). The game engine library (windjammer-game-core) retains the name `windjammer-app` for backward compatibility with existing dependencies.
