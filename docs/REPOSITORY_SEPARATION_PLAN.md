# Windjammer Repository Separation Plan

## Executive Summary

This document outlines the strategy for separating the Windjammer monorepo into multiple repositories to enable:
1. **Open-source UI framework** (public, MIT/Apache-2.0)
2. **Commercial game framework** (private, proprietary with free tier)
3. **Sustainable business model** (open-core + SaaS)

---

## Current State

### Single Monorepo Structure:
```
windjammer/
├── crates/
│   ├── windjammer/                    # Main language crate
│   ├── windjammer-compiler/           # Compiler
│   ├── windjammer-runtime/            # Runtime
│   ├── windjammer-ui/                 # UI framework ⭐ PUBLIC
│   ├── game-host-framework/     # Game framework 🔒 PRIVATE
│   ├── game-host-editor/        # Desktop editor 🔒 PRIVATE
│   ├── windjammer-editor-web/         # Browser editor 🔒 PRIVATE
│   ├── windjammer-c-ffi/              # FFI layer 🔒 PRIVATE
│   └── ...
├── sdks/                              # Multi-language SDKs 🔒 PRIVATE
└── docs/                              # Documentation
```

---

## Target State

### Three Separate Repositories:

#### 1. **windjammer** (Public - MIT/Apache-2.0)
The programming language and core tooling.

```
windjammer/
├── crates/
│   ├── windjammer/           # Language crate
│   ├── windjammer-compiler/  # Compiler
│   ├── windjammer-runtime/   # Runtime
│   ├── windjammer-lsp/       # Language server
│   └── windjammer-cli/       # CLI tools
├── docs/                     # Language documentation
├── examples/                 # Language examples
└── README.md
```

**Purpose**: Open-source programming language
**License**: MIT/Apache-2.0
**Target Audience**: General developers, language enthusiasts

---

#### 2. **windjammer-ui** (Public - MIT/Apache-2.0)
The declarative UI framework.

```
windjammer-ui/
├── crates/
│   ├── windjammer-ui/        # Core UI framework
│   ├── windjammer-ui-web/    # Web backend (HTML/CSS)
│   ├── windjammer-ui-native/ # Native backend (egui/iced)
│   └── windjammer-ui-gpu/    # GPU backend (wgpu)
├── examples/                 # UI examples
├── docs/                     # UI documentation
└── README.md
```

**Purpose**: Open-source UI framework (compete with React, Flutter)
**License**: MIT/Apache-2.0
**Target Audience**: Web/app developers, UI designers
**Value Proposition**: "Write once, run everywhere" UI framework

---

#### 3. **game-host** (Private - Proprietary)
The commercial game framework.

```
game-host/
├── crates/
│   ├── game-host-framework/  # Core game engine
│   ├── game-host-editor/     # Desktop editor
│   ├── windjammer-editor-web/      # Browser editor
│   ├── windjammer-c-ffi/           # FFI layer
│   └── game-host-runtime/    # Game runtime
├── sdks/                           # Multi-language SDKs
│   ├── python/
│   ├── javascript/
│   ├── csharp/
│   └── ... (12 languages)
├── docs/                           # Game framework docs
├── examples/                       # Game examples
└── README.md
```

**Purpose**: Commercial game framework
**License**: Proprietary (with free tier)
**Target Audience**: Game developers
**Value Proposition**: "Unity alternative with no runtime fees"

---

## Separation Strategy

### Phase 1: Planning (1 week)
- [ ] Finalize repository structure
- [ ] Define licensing for each repo
- [ ] Plan dependency management
- [ ] Design CI/CD for multiple repos
- [ ] Create migration scripts

### Phase 2: Extract windjammer-ui (1 week)
1. Create new `windjammer-ui` repository
2. Extract UI framework crates
3. Set up CI/CD
4. Update documentation
5. Publish to crates.io
6. Create landing page

### Phase 3: Extract game-host (1 week)
1. Create private `game-host` repository
2. Extract game framework crates
3. Extract SDKs
4. Set up private CI/CD
5. Update documentation
6. Configure access control

### Phase 4: Clean up windjammer (1 week)
1. Remove extracted crates
2. Update dependencies
3. Update documentation
4. Update README
5. Prepare for public release

### Phase 5: Testing & Validation (1 week)
1. Test all three repos independently
2. Verify cross-repo dependencies
3. Test CI/CD pipelines
4. Validate documentation
5. Test SDK builds

---

## Dependency Management

### Cross-Repository Dependencies:

```
windjammer (public)
    ↓
windjammer-ui (public)
    ↓
game-host (private)
```

**Rules**:
- Public repos can only depend on other public repos
- Private repos can depend on public repos
- No circular dependencies

**Implementation**:
- Use `git submodules` or `cargo workspaces`
- Publish public crates to crates.io
- Use path dependencies for development
- Use version dependencies for production

---

## Licensing Strategy

### windjammer (Language)
**License**: Dual MIT/Apache-2.0
**Rationale**: Standard for Rust projects, maximum adoption

### windjammer-ui (UI Framework)
**License**: Dual MIT/Apache-2.0
**Rationale**: Compete with React/Flutter, need permissive license

### game-host (Game Framework)
**License**: Proprietary with Free Tier
**Rationale**: Sustainable business model

**Free Tier**:
- ✅ Full engine access
- ✅ All 12 language SDKs
- ✅ Desktop & browser editors
- ✅ Community support
- ✅ No runtime fees
- ✅ No revenue limits
- ❌ No source code access
- ❌ No custom modifications

**Pro Tier** ($99/month or $999/year):
- ✅ Everything in Free
- ✅ Full source code access
- ✅ Custom engine modifications
- ✅ Priority support
- ✅ Advanced features (console export, etc.)
- ✅ Commercial license

**Enterprise Tier** (Custom pricing):
- ✅ Everything in Pro
- ✅ Dedicated support
- ✅ Custom features
- ✅ SLA guarantees
- ✅ Training & consulting

---

## Monetization Strategy

### Revenue Streams:

#### 1. **Game Framework Licensing** (Primary)
- Free tier: $0 (unlimited users)
- Pro tier: $99/month per developer
- Enterprise: Custom pricing

**Target**: 10,000 free users → 1,000 pro users = $1.2M ARR

#### 2. **SaaS Services** (Secondary)
- **Multiplayer Hosting**: $10-100/month per game
- **Analytics Dashboard**: $20/month per game
- **Asset Marketplace**: 30% commission
- **Cloud Builds**: $5 per 100 builds

**Target**: 500 paying games = $300K ARR

#### 3. **Support & Consulting** (Tertiary)
- **Priority Support**: Included in Pro/Enterprise
- **Custom Development**: $200/hour
- **Training**: $5,000 per team
- **Consulting**: Custom pricing

**Target**: 50 clients = $500K ARR

#### 4. **Marketplace** (Future)
- **Asset Sales**: 30% commission
- **Plugin Sales**: 30% commission
- **Template Sales**: 30% commission

**Target**: $1M GMV = $300K revenue

**Total Potential ARR**: $2.3M+

---

## Open-Core Model

### What's Open Source:
- ✅ Windjammer language
- ✅ Windjammer compiler
- ✅ Windjammer runtime
- ✅ Windjammer-UI framework
- ✅ Language documentation
- ✅ UI documentation

### What's Proprietary:
- 🔒 Game framework (engine)
- 🔒 Multi-language SDKs
- 🔒 Desktop editor
- 🔒 Browser editor
- 🔒 C FFI layer
- 🔒 Advanced features

### Why This Works:
1. **Language adoption**: Open-source language drives adoption
2. **UI framework adoption**: Open-source UI attracts web/app developers
3. **Game framework revenue**: Game developers pay for professional tools
4. **Network effects**: More language users → more potential game devs
5. **Sustainable**: Revenue funds continued development

---

## Migration Path for Users

### Existing Users (Pre-Separation):
- ✅ Automatic migration to new repos
- ✅ No breaking changes
- ✅ Free tier for all existing users
- ✅ 6-month grace period

### New Users (Post-Separation):
- Start with free tier
- Upgrade to Pro when needed
- Clear upgrade path

---

## Technical Implementation

### Repository Setup:

#### 1. Create Repositories:
```bash
# Create public repos
gh repo create windjammer/windjammer --public
gh repo create windjammer/windjammer-ui --public

# Create private repo
gh repo create windjammer/game-host --private
```

#### 2. Extract Crates:
```bash
# Extract windjammer-ui
git subtree split -P crates/windjammer-ui -b windjammer-ui-branch
cd ../windjammer-ui
git pull ../windjammer windjammer-ui-branch

# Extract game-host
git subtree split -P crates/game-host-framework -b game-branch
cd ../game-host
git pull ../game host-branch
```

#### 3. Update Dependencies:
```toml
# In game-host/Cargo.toml
[dependencies]
windjammer = "0.1"
windjammer-ui = "0.1"

# In windjammer-ui/Cargo.toml
[dependencies]
windjammer = "0.1"
```

#### 4. Set Up CI/CD:
- GitHub Actions for public repos
- Private CI for game framework
- Automated testing
- Automated publishing

---

## Timeline

### Week 1: Planning
- Finalize structure
- Create repositories
- Set up CI/CD templates

### Week 2: Extract windjammer-ui
- Extract crates
- Update dependencies
- Test builds
- Publish to crates.io

### Week 3: Extract game-host
- Extract crates
- Extract SDKs
- Update dependencies
- Test builds

### Week 4: Clean up & Test
- Clean up main repo
- Test all repos
- Update documentation
- Validate everything

### Week 5: Launch
- Announce separation
- Update website
- Publish blog post
- Notify users

**Total**: 5 weeks to complete separation

---

## Risks & Mitigation

### Risk 1: Breaking Changes
**Mitigation**: Extensive testing, gradual migration, compatibility layer

### Risk 2: User Confusion
**Mitigation**: Clear documentation, migration guide, FAQ

### Risk 3: Dependency Hell
**Mitigation**: Careful version management, automated testing

### Risk 4: Revenue Uncertainty
**Mitigation**: Free tier ensures adoption, gradual pricing rollout

### Risk 5: Competition
**Mitigation**: Superior tech, better pricing, open-source goodwill

---

## Success Metrics

### Year 1 Goals:
- 10,000+ free tier users
- 500+ pro tier users
- $600K ARR
- 100+ games published

### Year 2 Goals:
- 50,000+ free tier users
- 2,000+ pro tier users
- $2.4M ARR
- 1,000+ games published

### Year 3 Goals:
- 200,000+ free tier users
- 5,000+ pro tier users
- $6M ARR
- 10,000+ games published

---

## Next Steps

1. **Review this plan** with stakeholders
2. **Finalize licensing** terms
3. **Create migration scripts**
4. **Set up new repositories**
5. **Begin extraction** (Week 1)

---

## Conclusion

Repository separation is **critical** for:
- ✅ Open-source adoption (language + UI)
- ✅ Sustainable business model (game framework)
- ✅ Clear value proposition
- ✅ Professional image
- ✅ Investor appeal

**Recommendation**: Proceed with separation ASAP

---

*Document Version: 1.0*
*Last Updated: November 20, 2024*
*Status: DRAFT - Awaiting Approval*

