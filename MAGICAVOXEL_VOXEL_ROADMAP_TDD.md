# MagicaVoxel-Quality Voxel Rendering for Windjammer - TDD Roadmap

## 🎯 **Vision: Bring MagicaVoxel Visual Fidelity to Windjammer**

**Goal**: Integrate the proven voxel rendering system from `action-adventure-framework` (Godot/Go) into the Windjammer game engine, enabling gothic cathedrals, cyberpunk cities, and industrial complexes with stunning visual quality.

---

## 📊 **What We Have vs. What We Need**

### ✅ **What Exists (Proven in Godot Framework)**
- **SVO (Sparse Voxel Octree)** rendering system
- **Greedy meshing** algorithm (300ms for 48m × 48m terrain)
- **GPU raymarching** shaders (real-time voxel rendering)
- **LOD system** (27x performance improvement)
- **MagicaVoxel lighting** (SSAO, bloom, fog, shadows)
- **640 voxels/meter** resolution (26.4M voxels for characters!)
- **Complete TDD suite** (all features tested)

### ❌ **What Windjammer Needs**
- Voxel data structures (octree, grid)
- Voxel meshing algorithms
- Voxel rendering integration
- Lighting system upgrades
- Performance optimizations

---

## 🏗️ **Architecture Strategy**

### **Approach: Progressive Port with TDD**

We'll port the voxel system from Go/Godot to Windjammer in stages, using TDD at every step:

```
Phase 1: Voxel Data Structures (Windjammer)
Phase 2: Greedy Meshing (Windjammer)
Phase 3: SVO Octree (Windjammer)
Phase 4: Rendering Integration (windjammer-game)
Phase 5: GPU Raymarching (windjammer-game)
Phase 6: MagicaVoxel Lighting (windjammer-game)
```

---

## 📅 **Phase 1: Voxel Data Structures (Week 1)**

### **Goal**: Basic voxel grid in Windjammer

### **TDD Cycle 1.1: VoxelGrid**

**RED - Test First:**
```windjammer
// tests/voxel_grid_test.wj
@test
fn test_voxel_grid_creation() {
    let grid = VoxelGrid::new(16, 16, 16);
    assert_eq!(grid.width(), 16);
    assert_eq!(grid.height(), 16);
    assert_eq!(grid.depth(), 16);
}

@test
fn test_voxel_set_get() {
    let mut grid = VoxelGrid::new(8, 8, 8);
    grid.set(2, 3, 4, 255); // Set voxel to solid
    assert_eq!(grid.get(2, 3, 4), 255);
    assert_eq!(grid.get(0, 0, 0), 0); // Empty by default
}

@test
fn test_voxel_bounds_check() {
    let grid = VoxelGrid::new(8, 8, 8);
    assert!(!grid.is_valid(8, 0, 0)); // Out of bounds
    assert!(grid.is_valid(7, 7, 7));  // Valid
}
```

**GREEN - Implementation:**
```windjammer
// windjammer-game/windjammer-game-core/src_wj/voxel/grid.wj
struct VoxelGrid {
    width: i32,
    height: i32,
    depth: i32,
    data: Vec<u8>, // Flat array: index = x + y*width + z*width*height
}

impl VoxelGrid {
    fn new(width: i32, height: i32, depth: i32) -> VoxelGrid {
        let size = width * height * depth;
        VoxelGrid {
            width,
            height,
            depth,
            data: vec![0; size as usize],
        }
    }
    
    fn get(self, x: i32, y: i32, z: i32) -> u8 {
        if !self.is_valid(x, y, z) {
            return 0;
        }
        let idx = x + y * self.width + z * self.width * self.height;
        self.data[idx as usize]
    }
    
    fn set(self, x: i32, y: i32, z: i32, value: u8) {
        if !self.is_valid(x, y, z) {
            return;
        }
        let idx = x + y * self.width + z * self.width * self.height;
        self.data[idx as usize] = value;
    }
    
    fn is_valid(self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < self.width &&
        y >= 0 && y < self.height &&
        z >= 0 && z < self.depth
    }
    
    fn width(self) -> i32 { self.width }
    fn height(self) -> i32 { self.height }
    fn depth(self) -> i32 { self.depth }
}
```

**REFACTOR**: Clean up, optimize flat array indexing.

---

### **TDD Cycle 1.2: VoxelColor**

**RED - Test First:**
```windjammer
@test
fn test_voxel_color_creation() {
    let color = VoxelColor::new(255, 128, 64, 255);
    assert_eq!(color.r, 255);
    assert_eq!(color.g, 128);
    assert_eq!(color.b, 64);
    assert_eq!(color.a, 255);
}

@test
fn test_voxel_color_from_hex() {
    let color = VoxelColor::from_hex(0xFF8040FF);
    assert_eq!(color.r, 255);
    assert_eq!(color.g, 128);
    assert_eq!(color.b, 64);
    assert_eq!(color.a, 255);
}
```

**GREEN - Implementation:**
```windjammer
struct VoxelColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl VoxelColor {
    fn new(r: u8, g: u8, b: u8, a: u8) -> VoxelColor {
        VoxelColor { r, g, b, a }
    }
    
    fn from_hex(hex: u32) -> VoxelColor {
        VoxelColor {
            r: ((hex >> 24) & 0xFF) as u8,
            g: ((hex >> 16) & 0xFF) as u8,
            b: ((hex >> 8) & 0xFF) as u8,
            a: (hex & 0xFF) as u8,
        }
    }
    
    fn to_hex(self) -> u32 {
        ((self.r as u32) << 24) |
        ((self.g as u32) << 16) |
        ((self.b as u32) << 8) |
        (self.a as u32)
    }
}
```

---

## 📅 **Phase 2: Greedy Meshing (Week 2)**

### **Goal**: Convert voxel grid to optimized mesh

### **TDD Cycle 2.1: Face Extraction**

**RED - Test First:**
```windjammer
@test
fn test_extract_visible_faces() {
    let mut grid = VoxelGrid::new(3, 3, 3);
    // Create a single solid voxel in the center
    grid.set(1, 1, 1, 255);
    
    let faces = extract_visible_faces(grid);
    // 6 faces (top, bottom, left, right, front, back)
    assert_eq!(faces.len(), 6);
}

@test
fn test_hidden_face_culling() {
    let mut grid = VoxelGrid::new(3, 3, 3);
    // Fill completely (all faces should be culled)
    for x in 0..3 {
        for y in 0..3 {
            for z in 0..3 {
                grid.set(x, y, z, 255);
            }
        }
    }
    
    let faces = extract_visible_faces(grid);
    // Only exterior faces visible (54 total: 9 per side × 6 sides)
    assert_eq!(faces.len(), 54);
}
```

**GREEN - Implementation:**
```windjammer
struct VoxelFace {
    x: i32,
    y: i32,
    z: i32,
    direction: Direction, // +X, -X, +Y, -Y, +Z, -Z
    color: VoxelColor,
}

enum Direction {
    PosX, NegX,
    PosY, NegY,
    PosZ, NegZ,
}

fn extract_visible_faces(grid: VoxelGrid) -> Vec<VoxelFace> {
    let mut faces = Vec::new();
    
    for x in 0..grid.width() {
        for y in 0..grid.height() {
            for z in 0..grid.depth() {
                let voxel = grid.get(x, y, z);
                if voxel == 0 { continue; } // Empty voxel
                
                // Check each direction
                if grid.get(x + 1, y, z) == 0 {
                    faces.push(VoxelFace { x, y, z, direction: Direction::PosX, color: VoxelColor::from_hex(voxel as u32) });
                }
                // ... repeat for all 6 directions
            }
        }
    }
    
    faces
}
```

---

### **TDD Cycle 2.2: Greedy Meshing**

**RED - Test First:**
```windjammer
@test
fn test_greedy_merge_horizontal() {
    // Create 3 solid voxels in a row
    let mut grid = VoxelGrid::new(5, 5, 5);
    grid.set(1, 1, 1, 255);
    grid.set(2, 1, 1, 255);
    grid.set(3, 1, 1, 255);
    
    let mesh = greedy_mesh(grid);
    // Should merge into 1 quad (not 3 separate quads)
    assert!(mesh.quad_count() < 18); // Less than 3 voxels × 6 faces
}

@test
fn test_greedy_mesh_performance() {
    let grid = generate_test_terrain(64, 64, 64);
    let start = std::time::Instant::now();
    let mesh = greedy_mesh(grid);
    let elapsed = start.elapsed();
    
    // Should be fast (< 1 second for 64³ grid)
    assert!(elapsed.as_millis() < 1000);
    println("Greedy meshing 64³ grid: {}ms", elapsed.as_millis());
}
```

**GREEN - Implementation:**
```windjammer
struct VoxelMesh {
    vertices: Vec<Vec3>,
    normals: Vec<Vec3>,
    colors: Vec<Vec3>,
    indices: Vec<u32>,
}

fn greedy_mesh(grid: VoxelGrid) -> VoxelMesh {
    let mut mesh = VoxelMesh::new();
    
    // Process each axis (X, Y, Z)
    for axis in 0..3 {
        // Sweep through slices
        for slice in 0..grid.size(axis) {
            let faces = extract_slice_faces(grid, axis, slice);
            let quads = merge_faces_greedy(faces);
            mesh.add_quads(quads);
        }
    }
    
    mesh
}

fn merge_faces_greedy(faces: Vec<VoxelFace>) -> Vec<Quad> {
    // Greedy algorithm: merge adjacent faces of same color
    // 1. Sort faces by position
    // 2. Try to expand each face horizontally
    // 3. Try to expand vertically
    // 4. Output merged quad
    
    // ... implementation from action-adventure-framework
}
```

---

## 📅 **Phase 3: SVO Octree (Week 3)**

### **Goal**: Sparse voxel octree for massive scenes

### **TDD Cycle 3.1: Octree Node**

**RED - Test First:**
```windjammer
@test
fn test_octree_node_creation() {
    let node = OctreeNode::new();
    assert!(node.is_leaf());
    assert_eq!(node.value(), 0);
}

@test
fn test_octree_subdivision() {
    let mut node = OctreeNode::new();
    node.subdivide();
    assert!(!node.is_leaf());
    assert_eq!(node.child_count(), 8);
}
```

**GREEN - Implementation:**
```windjammer
struct OctreeNode {
    value: u8,      // Voxel value (0 = empty)
    children: Option<Box<[OctreeNode; 8]>>, // 8 octants
}

impl OctreeNode {
    fn new() -> OctreeNode {
        OctreeNode { value: 0, children: None }
    }
    
    fn is_leaf(self) -> bool {
        self.children.is_none()
    }
    
    fn subdivide(self) {
        if !self.is_leaf() { return; }
        
        self.children = Some(Box::new([
            OctreeNode::new(),
            OctreeNode::new(),
            OctreeNode::new(),
            OctreeNode::new(),
            OctreeNode::new(),
            OctreeNode::new(),
            OctreeNode::new(),
            OctreeNode::new(),
        ]));
    }
}
```

---

### **TDD Cycle 3.2: Grid to Octree Conversion**

**RED - Test First:**
```windjammer
@test
fn test_grid_to_octree() {
    let mut grid = VoxelGrid::new(8, 8, 8);
    grid.set(3, 3, 3, 255); // Single solid voxel
    
    let octree = Octree::from_grid(grid);
    assert_eq!(octree.get(3, 3, 3), 255);
    assert_eq!(octree.get(0, 0, 0), 0);
}

@test
fn test_octree_memory_efficiency() {
    // Sparse grid (mostly empty)
    let mut grid = VoxelGrid::new(128, 128, 128);
    for i in 0..100 {
        grid.set(i, i, i, 255);
    }
    
    let octree = Octree::from_grid(grid);
    
    // Octree should use much less memory than full grid
    let grid_memory = 128 * 128 * 128; // 2MB
    let octree_memory = octree.node_count() * size_of::<OctreeNode>();
    
    assert!(octree_memory < grid_memory / 10); // At least 10x compression
}
```

---

## 📅 **Phase 4: Rendering Integration (Week 4)**

### **Goal**: Display voxels in windjammer-game

### **TDD Cycle 4.1: Mesh to Render Buffer**

**RED - Test First:**
```windjammer
@test
fn test_voxel_mesh_to_gpu() {
    let mut grid = VoxelGrid::new(16, 16, 16);
    fill_test_pattern(grid);
    
    let mesh = greedy_mesh(grid);
    let render_data = mesh.to_render_buffer();
    
    // Verify vertex buffer format
    assert!(render_data.vertices.len() > 0);
    assert_eq!(render_data.vertices.len(), render_data.normals.len());
    assert_eq!(render_data.vertices.len(), render_data.colors.len());
    
    // Verify indices
    assert!(render_data.indices.len() > 0);
    assert_eq!(render_data.indices.len() % 3, 0); // Triangles
}
```

---

## 📅 **Phase 5: GPU Raymarching (Week 5)**

### **Goal**: Real-time voxel rendering via GPU compute shaders

### **TDD Cycle 5.1: Octree GPU Upload**

**RED - Test First:**
```windjammer
@test
fn test_octree_gpu_serialization() {
    let octree = create_test_octree(64, 64, 64);
    let gpu_data = octree.serialize_for_gpu();
    
    // Should be compact representation
    assert!(gpu_data.len() < 64 * 64 * 64); // More efficient than grid
    
    // Verify deserializable
    let reconstructed = Octree::deserialize_from_gpu(gpu_data);
    assert_eq!(octree.get(32, 32, 32), reconstructed.get(32, 32, 32));
}
```

---

## 📅 **Phase 6: MagicaVoxel Lighting (Week 6)**

### **Goal**: SSAO, bloom, fog, dramatic shadows

### **TDD Cycle 6.1: SSAO (Screen Space Ambient Occlusion)**

**RED - Test First:**
```windjammer
@test
fn test_ssao_enabled() {
    let scene = create_voxel_scene();
    let renderer = VoxelRenderer::new();
    
    renderer.enable_ssao(true);
    let frame = renderer.render(scene);
    
    // Verify SSAO is applied (darker crevices)
    let corner_pixel = frame.get_pixel(10, 10); // Corner voxel
    let flat_pixel = frame.get_pixel(100, 100); // Flat surface
    
    assert!(corner_pixel.brightness() < flat_pixel.brightness());
}
```

---

## 🎯 **Success Metrics**

### **Visual Quality** (Match MagicaVoxel References):
- [ ] Gothic cathedral with intricate stonework (640 v/m)
- [ ] Dramatic lighting (SSAO, shadows, fog)
- [ ] Cyberpunk neon glow (bloom, emissive materials)
- [ ] Industrial smoke/steam (volumetric effects)
- [ ] Expressive characters (individual fingernails, eyelashes)

### **Performance** (60+ FPS Target):
- [ ] Greedy meshing < 1 second for 64³ grid
- [ ] LOD system provides 27x speedup
- [ ] GPU raymarching achieves 60 FPS at 1080p
- [ ] Memory usage < 500MB for full scene

### **Competitive Position**:
- [ ] Matches MagicaVoxel visual quality
- [ ] Faster than manual 3D modeling workflow
- [ ] Enables procedural generation at scale
- [ ] Unique competitive advantage for Windjammer

---

## 📝 **Implementation Plan**

### **Week 1-2: Core Data Structures**
- VoxelGrid, VoxelColor (Phase 1)
- Face extraction, greedy meshing (Phase 2)
- All tests passing ✅

### **Week 3-4: Octree & Rendering**
- SVO octree implementation (Phase 3)
- Rendering integration (Phase 4)
- First voxel scenes rendering in Windjammer! 🎉

### **Week 5-6: GPU & Lighting**
- GPU raymarching shaders (Phase 5)
- MagicaVoxel lighting (Phase 6)
- **Visual quality matches references!** ✨

---

## 🔬 **Research Questions**

1. **Rust Interop**: Can we call existing Go voxel code from Windjammer?
   - Answer: Yes! Windjammer compiles to Rust, can call any Rust crate
   - Strategy: Port algorithms, not FFI (cleaner)

2. **GPU Compute**: Does Windjammer support compute shaders?
   - Answer: Through Rust interop (wgpu, vulkan crates)
   - Strategy: Start with CPU meshing, add GPU later

3. **Performance**: Can Windjammer match Go performance?
   - Answer: YES! Rust is as fast as Go (often faster)
   - Strategy: TDD ensures correctness, then optimize

---

## 💡 **Key Insights from Godot Framework**

### **What Made It Work:**
1. **LOD System** - 27x speedup, critical for gameplay
2. **Greedy Meshing** - Reduces triangle count by 90%+
3. **SVO Octree** - 10x memory compression for sparse scenes
4. **GPU Raymarching** - Enables unlimited voxel complexity
5. **TDD Throughout** - Every feature tested, no regressions

### **What We'll Improve:**
1. **Pure Windjammer** - No Godot dependency
2. **Better Performance** - Rust optimizations
3. **Cleaner API** - Windjammer's ergonomic syntax
4. **Cross-Platform** - Rust's portability

---

## 🎮 **Use Cases Enabled**

### **Gothic Cathedral** (Image 1 & 3):
- 640 v/m resolution for stonework detail
- SSAO for depth in crevices
- Volumetric light shafts through windows
- Weathered stone texture variation

### **Cyberpunk City** (Image 7):
- Bloom for neon glow effects
- Emissive materials for signs/lights
- Grungy metal with roughness variation
- Atmospheric fog (colored, not just grey)

### **Industrial Complex** (Image 4):
- Volumetric smoke/steam effects
- Complex machinery (catwalks, pipes)
- Dramatic shadows (hard light + soft AO)
- Massive scale structures

### **Expressive Characters**:
- Facial expressions (26.4M voxels)
- Detailed armor (visible rivets)
- Smooth animations via LOD
- Individual fingernails, eyelashes!

---

## 📸 **Validation Strategy**

### **TDD at Every Step:**
1. **RED**: Write test that fails
2. **GREEN**: Implement minimal fix
3. **REFACTOR**: Clean up code
4. **VERIFY**: Compare to MagicaVoxel references

### **Visual Comparison:**
- Screenshot Windjammer voxel scenes
- Place side-by-side with MagicaVoxel references
- Use 23-persona evaluation for quality check
- Iterate until quality matches!

---

## 🚀 **Next Session Actions**

1. **Start Phase 1** - VoxelGrid implementation
2. **Write first TDD test** - `test_voxel_grid_creation()`
3. **Make it pass** - Implement VoxelGrid struct
4. **Commit & push** - Document progress

---

**Let's build worlds with MagicaVoxel quality in Windjammer!** 🎨✨

---

*Roadmap created: 2026-02-22*  
*Status: Ready to start Phase 1 (VoxelGrid)*  
*Estimated timeline: 6 weeks to full MagicaVoxel quality*  
*Confidence: HIGH - proven algorithms, mature TDD process*
