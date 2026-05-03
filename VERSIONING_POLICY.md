# Windjammer Versioning Policy

## Current Versions

### Windjammer Compiler: 0.46.0

**Components at 0.46.0:**
- `windjammer` (compiler)
- `windjammer-runtime` (standard library)
- `windjammer-lsp` (language server, if exists)
- All other compiler child crates

### Separate Projects (Independent Versioning)

**Game Host Engine: 0.1.0**
- `game-host` (game engine library)
- `game-host-core` (core engine)
- `windjammer-runtime-host` (runtime)
- `wj-game` (build plugin)

**Sample Game: 0.1.0**
- `sample-game` (game binary)
- All sample-game crates

## Versioning Independence

**IMPORTANT:** These are SEPARATE projects:
1. **Windjammer** = Compiler (language tooling)
2. **game-host** = Game engine (library using Windjammer)
3. **Sample Game** = Reference game (dogfooding project)

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
| Game Engine | `windjammer-app` (game-host-core) | N/A (library) |
| Runtime Host | `sample-game-host` | `sample-game-host` |

**Historical note:** Compiler was incorrectly named `windjammer-app` (fixed 2026-03-14). The game engine library (game-host-core) retains the name `windjammer-app` for backward compatibility with existing dependencies.
