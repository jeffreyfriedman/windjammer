# SDK Testing Plan - Ensuring Every Example is a Playable Game

**Critical Requirement**: Every example in every language must be a **fully working, playable game** before public release.

---

## 🎯 Testing Philosophy

### Zero Tolerance for Broken Examples
- ❌ No stub implementations
- ❌ No "TODO" placeholders
- ❌ No compilation errors
- ❌ No runtime crashes
- ✅ **Every example must be playable**
- ✅ **Every example must demonstrate real features**
- ✅ **Every example must be fun (or at least functional)**

### Quality Standards
1. **Compiles**: No compilation/syntax errors
2. **Runs**: No runtime crashes or exceptions
3. **Playable**: User can interact with the game
4. **Complete**: All features work as advertised
5. **Performant**: Runs at 60 FPS minimum
6. **Cross-platform**: Works on Windows, macOS, Linux

---

## 📋 Testing Matrix

### Languages to Test (12 total)
1. ✅ Python
2. ✅ JavaScript
3. ✅ TypeScript
4. ✅ C#
5. ✅ C++
6. ✅ Rust
7. ✅ Go
8. ✅ Java
9. ✅ Kotlin
10. ✅ Lua
11. ✅ Swift
12. ✅ Ruby

### Example Games per Language (Minimum 3)
1. **Hello World** - Basic window, sprite, input
2. **2D Platformer** - Player movement, jumping, collisions
3. **3D Scene** - 3D camera, mesh rendering, lighting

### Platforms to Test (3 minimum)
1. **Windows** - x86_64
2. **macOS** - x86_64 + ARM64
3. **Linux** - x86_64

**Total Test Cases**: 12 languages × 3 examples × 3 platforms = **108 test cases minimum**

---

## 🔬 Testing Phases

### Phase 1: Manual Testing (Per Language)

#### Step 1: Setup Environment
```bash
# Example for Python
cd examples/python/hello_world
python -m venv venv
source venv/bin/activate  # or venv\Scripts\activate on Windows
pip install -r requirements.txt
```

#### Step 2: Compile/Build
```bash
# Language-specific build command
python hello_world.py  # Python
node hello_world.js    # JavaScript
tsc && node dist/hello_world.js  # TypeScript
dotnet run            # C#
cargo run             # Rust
go run hello_world.go # Go
# etc.
```

#### Step 3: Run Game
- Game window opens
- Graphics render correctly
- Input responds
- No crashes for 5 minutes of play

#### Step 4: Test Features
**Hello World**:
- [x] Window opens (800x600 default)
- [x] Sprite renders on screen
- [x] Sprite moves with arrow keys
- [x] ESC key closes game
- [x] No console errors
- [x] Runs at 60 FPS

**2D Platformer**:
- [x] Player sprite renders
- [x] Player moves left/right
- [x] Player jumps
- [x] Gravity works
- [x] Ground collision works
- [x] Player can't fall through floor
- [x] Runs at 60 FPS

**3D Scene**:
- [x] 3D camera works
- [x] Mesh renders with lighting
- [x] Camera rotates with mouse
- [x] Camera moves with WASD
- [x] No Z-fighting or artifacts
- [x] Runs at 60 FPS

#### Step 5: Document Results
```markdown
## Python - Hello World - Windows
- ✅ Compiles: Yes
- ✅ Runs: Yes
- ✅ Playable: Yes
- ✅ FPS: 120 (exceeds 60)
- ✅ Crashes: None
- ✅ Status: PASS
```

---

### Phase 2: Automated Testing

#### Docker-Based Testing
Create Docker containers for each language to ensure consistent environments:

```dockerfile
# Example: Dockerfile.python
FROM python:3.11-slim

WORKDIR /app
COPY requirements.txt .
RUN pip install -r requirements.txt

COPY examples/python/hello_world/ .

# Run tests
CMD ["python", "test_hello_world.py"]
```

#### Test Script Template
```python
# test_hello_world.py
import subprocess
import time
import psutil

def test_game_runs():
    """Test that game starts and runs without crashing"""
    process = subprocess.Popen(['python', 'hello_world.py'])
    time.sleep(5)  # Let it run for 5 seconds
    
    # Check if still running
    assert process.poll() is None, "Game crashed!"
    
    # Check memory usage (shouldn't leak)
    proc = psutil.Process(process.pid)
    memory_mb = proc.memory_info().rss / 1024 / 1024
    assert memory_mb < 500, f"Memory usage too high: {memory_mb}MB"
    
    # Check CPU usage (shouldn't be 100%)
    cpu_percent = proc.cpu_percent(interval=1)
    assert cpu_percent < 90, f"CPU usage too high: {cpu_percent}%"
    
    # Clean shutdown
    process.terminate()
    process.wait(timeout=5)
    
    print("✅ Test passed!")

if __name__ == "__main__":
    test_game_runs()
```

#### CI/CD Integration
```yaml
# .github/workflows/test-sdks.yml
name: Test All SDK Examples

on: [push, pull_request]

jobs:
  test-python:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - uses: actions/checkout@v3
      - uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      - name: Test Hello World
        run: |
          cd examples/python/hello_world
          pip install -r requirements.txt
          python test_hello_world.py
      - name: Test 2D Platformer
        run: |
          cd examples/python/platformer_2d
          pip install -r requirements.txt
          python test_platformer.py
  
  test-javascript:
    # Similar for JavaScript...
  
  # ... repeat for all 12 languages
```

---

### Phase 3: Performance Testing

#### Benchmark Each Example
```python
# benchmark.py
import time
import statistics

def benchmark_game(game_script, duration=60):
    """Run game for 60 seconds and measure FPS"""
    fps_samples = []
    
    # Run game and collect FPS data
    # (Implementation depends on game framework)
    
    avg_fps = statistics.mean(fps_samples)
    min_fps = min(fps_samples)
    p95_fps = statistics.quantiles(fps_samples, n=20)[18]  # 95th percentile
    
    print(f"Average FPS: {avg_fps:.1f}")
    print(f"Minimum FPS: {min_fps:.1f}")
    print(f"95th Percentile FPS: {p95_fps:.1f}")
    
    # Assert performance requirements
    assert avg_fps >= 60, f"Average FPS too low: {avg_fps}"
    assert min_fps >= 30, f"Minimum FPS too low: {min_fps}"
    assert p95_fps >= 55, f"95th percentile FPS too low: {p95_fps}"
    
    return {
        'avg_fps': avg_fps,
        'min_fps': min_fps,
        'p95_fps': p95_fps
    }
```

#### Performance Comparison
Verify that all languages achieve **95%+ of native Rust performance**:

```python
# Compare Python vs Rust performance
rust_fps = benchmark_game('rust/hello_world')
python_fps = benchmark_game('python/hello_world.py')

performance_ratio = python_fps['avg_fps'] / rust_fps['avg_fps']
assert performance_ratio >= 0.95, f"Python performance too low: {performance_ratio*100:.1f}%"
```

---

### Phase 4: Cross-Platform Testing

#### Test Matrix
| Language | Windows | macOS | Linux | Status |
|----------|---------|-------|-------|--------|
| Python   | ✅      | ✅    | ✅    | PASS   |
| JavaScript | ✅    | ✅    | ✅    | PASS   |
| TypeScript | ✅    | ✅    | ✅    | PASS   |
| C#       | ✅      | ✅    | ✅    | PASS   |
| C++      | ✅      | ✅    | ✅    | PASS   |
| Rust     | ✅      | ✅    | ✅    | PASS   |
| Go       | ✅      | ✅    | ✅    | PASS   |
| Java     | ✅      | ✅    | ✅    | PASS   |
| Kotlin   | ✅      | ✅    | ✅    | PASS   |
| Lua      | ✅      | ✅    | ✅    | PASS   |
| Swift    | ❌      | ✅    | ❌    | macOS only |
| Ruby     | ✅      | ✅    | ✅    | PASS   |

#### Platform-Specific Issues
Document and fix any platform-specific bugs:
- Windows: Path separators (`\` vs `/`)
- macOS: ARM64 vs x86_64
- Linux: Different distros (Ubuntu, Fedora, Arch)

---

## 🐛 Bug Tracking

### Issue Template
```markdown
## Bug Report: [Language] - [Example] - [Platform]

**Description**: Brief description of the issue

**Steps to Reproduce**:
1. Clone repo
2. cd examples/python/hello_world
3. python hello_world.py
4. Press spacebar

**Expected Behavior**: Player should jump

**Actual Behavior**: Game crashes with error: ...

**Environment**:
- OS: Windows 11
- Language Version: Python 3.11.5
- Windjammer Version: 0.34.0

**Logs**:
```
[error logs here]
```

**Status**: 🔴 BLOCKING
```

### Priority Levels
- 🔴 **BLOCKING**: Game doesn't run at all
- 🟠 **CRITICAL**: Game runs but major feature broken
- 🟡 **HIGH**: Game playable but minor feature broken
- 🟢 **MEDIUM**: Cosmetic issue or performance problem
- 🔵 **LOW**: Enhancement or nice-to-have

---

## ✅ Acceptance Criteria

### Per Example
- [x] Compiles without errors
- [x] Runs without crashes (5 min minimum)
- [x] All advertised features work
- [x] Runs at 60 FPS minimum
- [x] Works on all 3 platforms (Windows, macOS, Linux)
- [x] Memory usage < 500MB
- [x] CPU usage < 90%
- [x] No console errors or warnings
- [x] Clean shutdown (no zombie processes)

### Per Language
- [x] All 3 examples pass acceptance criteria
- [x] Performance within 95% of Rust
- [x] Documentation is accurate
- [x] README has correct setup instructions
- [x] Dependencies are properly listed

### Overall SDK Release
- [x] All 12 languages pass acceptance criteria
- [x] All 36 examples (12 × 3) are playable
- [x] CI/CD pipeline passes for all examples
- [x] Performance benchmarks documented
- [x] Known issues documented
- [x] Migration guides tested with real games

---

## 📊 Testing Dashboard

### Current Status (Example)
```
SDK Testing Progress: 36/108 (33%)

Python:    ✅✅✅ (3/3 examples × 3/3 platforms = 9/9 ✅)
JavaScript: ✅✅⚠️ (3/3 examples × 2/3 platforms = 6/9 ⚠️)
TypeScript: ⏳⏳⏳ (0/3 examples × 0/3 platforms = 0/9 ⏳)
C#:        ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
C++:       ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Rust:      ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Go:        ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Java:      ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Kotlin:    ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Lua:       ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Swift:     ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)
Ruby:      ❌❌❌ (0/3 examples × 0/3 platforms = 0/9 ❌)

Legend:
✅ = All tests pass
⚠️ = Some tests fail (non-blocking)
⏳ = Testing in progress
❌ = Not yet tested
```

---

## 🚀 Testing Workflow

### Week 1-2: Create Examples
1. Implement Hello World for all 12 languages
2. Implement 2D Platformer for all 12 languages
3. Implement 3D Scene for all 12 languages

### Week 3-4: Manual Testing
1. Test each example on primary platform (developer machine)
2. Fix any blocking issues
3. Document known issues

### Week 5-6: Automated Testing
1. Create Docker containers for each language
2. Write automated test scripts
3. Set up CI/CD pipeline

### Week 7-8: Cross-Platform Testing
1. Test on Windows
2. Test on macOS
3. Test on Linux
4. Fix platform-specific issues

### Week 9-10: Performance Testing
1. Benchmark each example
2. Verify 95%+ native performance
3. Optimize if needed

### Week 11-12: Final Validation
1. Run full test suite
2. Fix any remaining issues
3. Update documentation
4. **RELEASE READY** ✅

---

## 📝 Test Reports

### Example Test Report
```markdown
# SDK Testing Report - Python

**Date**: 2025-11-20
**Tester**: QA Team
**Version**: 0.34.0

## Summary
- Examples Tested: 3
- Platforms Tested: 3
- Total Test Cases: 9
- Passed: 9
- Failed: 0
- **Status**: ✅ PASS

## Detailed Results

### Hello World
- Windows: ✅ PASS (120 FPS avg)
- macOS: ✅ PASS (115 FPS avg)
- Linux: ✅ PASS (118 FPS avg)

### 2D Platformer
- Windows: ✅ PASS (95 FPS avg)
- macOS: ✅ PASS (92 FPS avg)
- Linux: ✅ PASS (94 FPS avg)

### 3D Scene
- Windows: ✅ PASS (75 FPS avg)
- macOS: ✅ PASS (72 FPS avg)
- Linux: ✅ PASS (74 FPS avg)

## Performance Comparison
- Python vs Rust: 96% (exceeds 95% requirement ✅)

## Issues Found
- None

## Recommendations
- Python SDK ready for release ✅
```

---

## 🎯 Success Metrics

### Release Criteria
- [ ] 100% of examples compile
- [ ] 100% of examples run without crashes
- [ ] 100% of examples are playable
- [ ] 95%+ of examples meet performance targets
- [ ] 90%+ of examples work on all platforms
- [ ] 0 blocking bugs
- [ ] < 5 critical bugs (with workarounds)

### Quality Metrics
- **Code Coverage**: 95%+ for SDK code
- **Test Coverage**: 100% of examples tested
- **Platform Coverage**: 90%+ (some languages may be platform-specific)
- **Performance**: 95%+ of native Rust performance
- **User Satisfaction**: 4.5/5 stars minimum (post-release)

---

## 🔧 Tools & Infrastructure

### Testing Tools
- **Docker**: Consistent environments
- **GitHub Actions**: CI/CD automation
- **pytest**: Python testing
- **Jest**: JavaScript/TypeScript testing
- **NUnit**: C# testing
- **Google Test**: C++ testing
- **cargo test**: Rust testing
- **go test**: Go testing
- **JUnit**: Java/Kotlin testing
- **busted**: Lua testing
- **XCTest**: Swift testing
- **RSpec**: Ruby testing

### Monitoring Tools
- **Performance Profiler**: Built-in Windjammer profiler
- **Memory Profiler**: Valgrind, heaptrack
- **FPS Counter**: Built-in debug overlay
- **Crash Reporter**: Automated crash logs

---

## 📞 Support & Escalation

### If Tests Fail
1. **Document the issue** (use bug template)
2. **Assign priority** (blocking, critical, high, medium, low)
3. **Notify team** (Discord, Slack, email)
4. **Fix immediately** (if blocking)
5. **Retest** (verify fix)
6. **Update documentation** (if needed)

### Escalation Path
1. **Developer** → Fix within 24 hours (blocking) or 1 week (non-blocking)
2. **Team Lead** → Review and prioritize
3. **Project Manager** → Decide if release can proceed
4. **Community** → Report to users if release delayed

---

## 🎉 Conclusion

**Every example must be a playable game.** This is non-negotiable.

By following this comprehensive testing plan, we ensure that:
- ✅ Every developer can run our examples
- ✅ Every example demonstrates real features
- ✅ Every language gets equal quality
- ✅ Windjammer's reputation for quality is maintained

**Quality over speed. Always.** 🚀

---

**Next Steps**: Execute testing plan, fix all issues, release incredible SDKs! 🎮

