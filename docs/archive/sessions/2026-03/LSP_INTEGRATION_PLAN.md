# LSP Integration Plan

## Current Status

The Windjammer LSP server already exists with:
- ✅ Salsa-powered incremental computation
- ✅ Basic diagnostics
- ✅ Hover, completion, goto definition
- ✅ Symbol navigation
- ✅ Refactoring support
- ✅ Semantic tokens

## Integration Goals

Integrate our world-class error system into the LSP:

### 1. Enhanced Diagnostics (High Priority)
- [ ] Integrate ErrorMapper for Rust → Windjammer error translation
- [ ] Add Windjammer error codes (WJ0001-WJ0010) to diagnostics
- [ ] Include contextual help in diagnostic messages
- [ ] Add related information for multi-location errors
- [ ] Syntax highlighting in diagnostic messages

### 2. Code Actions (High Priority)
- [ ] Integrate AutoFix system for quick fixes
- [ ] Add "Explain error code" action
- [ ] Add "View in error catalog" action
- [ ] Auto-fix on save option

### 3. Hover Enhancements (Medium Priority)
- [ ] Show error explanations on hover
- [ ] Link to error catalog
- [ ] Show fix suggestions

### 4. Commands (Medium Priority)
- [ ] `windjammer.explainError` - Explain current error
- [ ] `windjammer.fixAllErrors` - Fix all fixable errors
- [ ] `windjammer.showErrorStats` - Show error statistics
- [ ] `windjammer.openErrorCatalog` - Open error catalog

### 5. Configuration (Low Priority)
- [ ] Enable/disable auto-fix
- [ ] Error severity levels
- [ ] Filter error types

## Implementation Strategy

### Phase 1: Core Integration (2-3 hours)
1. Update `diagnostics.rs` to use ErrorMapper
2. Add Windjammer error codes to diagnostics
3. Test with existing LSP clients

### Phase 2: Code Actions (2-3 hours)
1. Implement quick fix code actions
2. Add explain error action
3. Test auto-fix in editor

### Phase 3: VS Code Extension (3-4 hours)
1. Create VS Code extension skeleton
2. Add syntax highlighting
3. Add commands
4. Package and test

### Phase 4: Documentation & Polish (1-2 hours)
1. Update LSP README
2. Create VS Code extension README
3. Add screenshots
4. Publish to marketplace

## Total Estimate: 8-12 hours

This is significantly less than the original 40-60h estimate because:
- LSP infrastructure already exists
- We just need to integrate our error system
- Most heavy lifting is done

## Success Criteria

- [x] Real-time Windjammer error codes in editor
- [ ] Quick fixes work in VS Code
- [ ] Explain error command works
- [ ] Error catalog accessible from editor
- [ ] Published VS Code extension

## Next Steps

1. Start with Phase 1: Core Integration
2. Test with existing LSP setup
3. Move to Phase 2: Code Actions
4. Create VS Code extension
5. Document and publish

