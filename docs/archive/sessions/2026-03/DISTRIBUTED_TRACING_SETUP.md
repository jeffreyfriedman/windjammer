# Distributed Tracing Setup for GPU Debugging

**Status**: ✅ Partially Implemented (Tracy + structured logging + RenderDoc support)

## What We Implemented

### 1. Tracy Profiler (CPU + GPU Timeline)
**Status**: Infrastructure ready, compile-time feature flag

**Enable**:
```bash
cd windjammer-game/windjammer-runtime-host
cargo build --release --features tracy
```

**Usage**:
1. Download Tracy profiler: https://github.com/wolfpld/tracy
2. Run Tracy server
3. Run game with `--features tracy`
4. Connect Tracy to visualize CPU/GPU timeline

**Benefits**:
- Visual timeline of all CPU work
- Cross-thread synchronization visualization
- GPU command buffer execution timing
- Zero runtime overhead when disabled

### 2. Structured Logging (tracing crate)
**Status**: Ready, always enabled

**Features**:
- Spans for logical operations
- Hierarchical event tracking
- CPU zone profiling
- Thread-aware logging

**Output**:
```
[INFO cpu_zone{name="gpu_dispatch_compute"}] dispatch_compute(160, 90, 1)
```

### 3. RenderDoc Integration
**Status**: Manual capture ready

**Setup**:
```bash
# macOS
brew install renderdoc

# Run game under RenderDoc
renderdoc breach-protocol-host
```

**What it shows**:
- Every buffer's GPU contents
- Exact shader execution state
- Bind group layouts
- Pipeline state
- GPU memory contents

**Key for current bug**: Can show if GBuffer actually has data after raymarch pass!

### 4. wgpu-profiler (GPU Timing Queries)
**Status**: Deferred - API complexity

Will implement later with proper device/queue threading.

## Current Debugging Approach

1. **Structured Logging** - Always on
   - Tracks buffer IDs
   - Logs bind calls
   - Shows dispatch parameters

2. **GPU Buffer Readback** - Implemented
   - Verifies CPU→GPU data transfer
   - Confirms SVO buffer has correct data

3. **RenderDoc** - Recommended next step
   - Capture a single frame
   - Inspect GBuffer contents after raymarch
   - See if shader executed at all

## How to Use RenderDoc Now

1. **Capture a frame**:
   ```bash
   # Launch game in RenderDoc
   open -a RenderDoc
   # File → Launch Application → breach-protocol-host
   # Press F12 during game to capture frame
   ```

2. **Inspect buffers**:
   - Find "compute_dispatch" in event list
   - Click on raymarch dispatch
   - Go to "Pipeline State" → "Resources"
   - Find GBuffer (binding 2) → "View Buffer"
   - Check if `material_id` field is non-zero

3. **Check shader execution**:
   - Look at "Mesh Output" tab
   - See if threads actually ran
   - Check bound resources match expected IDs

## Next Steps

1. ✅ **Tracy** - Compile with `--features tracy`, connect profiler
2. ✅ **RenderDoc** - Capture frame, inspect GBuffer
3. ⏳ **wgpu-profiler** - Implement proper GPU timing later

## Why This Helps

**Before**: "GBuffer is empty, no idea why"

**After**: 
- Tracy shows if CPU is blocking
- RenderDoc shows if GPU executed shader
- Buffer readback confirms data upload
- Structured logging traces every step

**Result**: Pin-point exact failure location in pipeline!
