# Windjammer Scene Management Guide 🎬

## Overview

The Windjammer Game Editor includes a comprehensive scene management system for building 2D and 3D games. This guide covers all features and workflows.

## Scene Hierarchy Panel 🌳

### Features
- **Visual Object Tree**: Hierarchical view of all scene objects
- **2D/3D Mode Indicator**: Shows current scene mode (🎮 2D or 🎲 3D)
- **Object Icons**: Visual identification of object types
- **Selection**: Click to select objects for editing
- **Add Objects**: Menu-driven object creation
- **Remove Objects**: Delete selected objects

### Object Icons
- 🧊 Cube
- ⚪ Sphere
- ⬜ Plane
- 🥫 Cylinder
- 💊 Capsule
- 🖼️ Sprite
- 🗺️ TileMap
- ☀️ Directional Light
- 💡 Point Light
- 🔦 Spot Light
- 📷 Camera
- 📦 Empty (Container)

### Adding Objects

**3D Primitives**:
1. Click "➕ Add Object"
2. Select from 3D Primitives section:
   - Cube (default size: 1.0)
   - Sphere (default radius: 0.5)
   - Plane (default: 10x10)

**Lights**:
1. Click "➕ Add Object"
2. Select from Lights section:
   - Directional Light (sun/moon)
   - Point Light (bulb/torch)

**2D Objects**:
1. Click "➕ Add Object"
2. Select from 2D Objects section:
   - Sprite (texture-based)

### Removing Objects
1. Select object in hierarchy
2. Click "🗑️ Remove Selected"

## Properties Panel ⚙️

### Common Properties

**Name**: Editable text field for object identification

**Visibility**: Toggle to show/hide object in scene

**Transform**:
- **Position** (X, Y, Z): Object location in world space
- **Rotation** (X, Y, Z): Euler angles in degrees
- **Scale** (X, Y, Z): Object size multiplier (1.0 = normal)

### Object-Specific Properties

#### Cube 🧊
- **Size**: Edge length (0.1 - 10.0)

#### Sphere ⚪
- **Radius**: Sphere radius (0.1 - 10.0)

#### Plane ⬜
- **Width**: Horizontal dimension (0.1 - 100.0)
- **Height**: Vertical dimension (0.1 - 100.0)

#### Cylinder 🥫
- **Radius**: Base radius (0.1 - 10.0)
- **Height**: Cylinder height (0.1 - 20.0)

#### Capsule 💊
- **Radius**: Capsule radius (0.1 - 10.0)
- **Height**: Total height (0.1 - 20.0)

#### Sprite 🖼️
- **Texture**: Path to texture file
- **Width**: Sprite width in pixels (1 - 1000)
- **Height**: Sprite height in pixels (1 - 1000)

#### Directional Light ☀️
- **Color**: RGB color picker
- **Intensity**: Light strength (0.0 - 5.0)

#### Point Light 💡
- **Color**: RGB color picker
- **Intensity**: Light strength (0.0 - 5.0)
- **Range**: Light radius (1.0 - 100.0)

#### Spot Light 🔦
- **Color**: RGB color picker
- **Intensity**: Light strength (0.0 - 5.0)
- **Range**: Light radius (1.0 - 100.0)
- **Cone Angle**: Spotlight cone in degrees (1.0 - 180.0)

#### Camera 📷
- Properties managed globally (see Scene Settings)

#### Empty 📦
- Container for grouping objects
- No additional properties

## Scene Settings 🎮

### Mode
- **2D Mode**: Orthographic camera, 2D physics
- **3D Mode**: Perspective camera, 3D physics

### Camera (Global)
- **Position**: Camera location
- **FOV**: Field of view (30° - 120°)
- **Near Clip**: Minimum render distance
- **Far Clip**: Maximum render distance
- **Orthographic**: Toggle for 2D mode

### Lighting
- **Ambient Color**: Global ambient light color
- **Ambient Intensity**: Ambient light strength (0.0 - 1.0)

### Skybox
- **None**: No skybox
- **Solid Color**: Single color background
- **Gradient**: Two-color gradient (top/bottom)
- **Cubemap**: 6-sided texture (future)

### Physics
- **Enabled**: Toggle physics simulation
- **Gravity**: Gravity vector (X, Y, Z)

## Scene Serialization 💾

### Saving Scenes
```rust
scene.save_to_file("my_scene.json")?;
```

### Loading Scenes
```rust
let scene = Scene::load_from_file("my_scene.json")?;
```

### JSON Format
```json
{
  "name": "My Scene",
  "mode": "ThreeD",
  "objects": {
    "Camera_uuid": {
      "id": "Camera_uuid",
      "name": "Main Camera",
      "object_type": "Camera",
      "transform": {
        "position": { "x": 0.0, "y": 2.0, "z": 10.0 },
        "rotation": { "x": 0.0, "y": 0.0, "z": 0.0 },
        "scale": { "x": 1.0, "y": 1.0, "z": 1.0 }
      },
      "visible": true,
      "children": []
    }
  },
  "camera": {
    "position": { "x": 0.0, "y": 2.0, "z": 10.0 },
    "rotation": { "x": 0.0, "y": 0.0, "z": 0.0 },
    "fov": 60.0,
    "near": 0.1,
    "far": 1000.0,
    "orthographic": false
  },
  "lighting": {
    "ambient_color": { "r": 0.3, "g": 0.3, "b": 0.4, "a": 1.0 },
    "ambient_intensity": 0.2
  },
  "skybox": {
    "Gradient": {
      "top": { "r": 0.5, "g": 0.7, "b": 1.0, "a": 1.0 },
      "bottom": { "r": 0.8, "g": 0.9, "b": 1.0, "a": 1.0 }
    }
  },
  "physics": {
    "enabled": true,
    "gravity": { "x": 0.0, "y": -9.81, "z": 0.0 }
  }
}
```

## Workflows 🔄

### Creating a 3D Scene

1. **Create New Scene**
   - Default mode: 3D
   - Includes camera and directional light

2. **Add Ground Plane**
   - Add Object → Plane
   - Set scale: (10, 1, 10)
   - Position: (0, 0, 0)

3. **Add Objects**
   - Add cubes, spheres for greyboxing
   - Position and scale as needed

4. **Configure Lighting**
   - Adjust directional light rotation
   - Add point lights for accents
   - Set ambient lighting

5. **Set Skybox**
   - Choose gradient or solid color
   - Adjust colors for mood

6. **Save Scene**
   - File → Save Scene
   - Choose location and name

### Creating a 2D Scene

1. **Switch to 2D Mode**
   - Scene → Mode → 2D
   - Camera switches to orthographic

2. **Add Sprites**
   - Add Object → Sprite
   - Set texture path
   - Adjust width/height

3. **Add 2D Lights** (optional)
   - Point lights work in 2D
   - Position in front of sprites (Z > 0)

4. **Configure Physics**
   - Set gravity: (0, -9.81, 0) for platformers
   - Or (0, 0, 0) for top-down games

5. **Save Scene**

### Greyboxing a Level

1. **Create Base Geometry**
   - Use planes for floors
   - Use cubes for walls
   - Use cylinders for pillars

2. **Arrange Objects**
   - Position with transform tools
   - Use scale for sizing
   - Rotate for variety

3. **Add Lighting**
   - Directional light for sun
   - Point lights for interiors
   - Adjust intensity and color

4. **Test in Game**
   - Build and run
   - Iterate on layout

5. **Replace with Final Assets**
   - Swap primitives for models
   - Keep transforms

## Tips & Best Practices 💡

### Performance
- **Limit Lights**: Max 4-8 dynamic lights per scene
- **Use Ambient**: Reduce need for many lights
- **Cull Invisible**: Set visible=false for off-screen objects

### Organization
- **Name Objects**: Use descriptive names
- **Use Empty Containers**: Group related objects
- **Consistent Naming**: player_*, enemy_*, prop_*

### Lighting
- **One Directional**: Use one main directional light (sun)
- **Accent with Point**: Add point lights for highlights
- **Match Skybox**: Coordinate light colors with skybox

### 2D Games
- **Z-Ordering**: Use Z position for layer depth
- **Camera Distance**: Keep camera far enough to see scene
- **Sprite Sizing**: Match sprite dimensions to texture

### 3D Games
- **Scale Consistency**: Keep objects at realistic scales
- **Ground Plane**: Always have a ground reference
- **Camera Height**: Position camera at player eye level (1.6-2.0)

## Keyboard Shortcuts ⌨️

| Action | macOS | Windows/Linux |
|--------|-------|---------------|
| Save Scene | Cmd+S | Ctrl+S |
| New Scene | Cmd+N | Ctrl+N |
| Delete Object | Delete | Delete |
| Duplicate Object | Cmd+D | Ctrl+D |
| Focus Object | F | F |

## API Reference 📚

### Scene
```rust
pub struct Scene {
    pub name: String,
    pub mode: SceneMode,
    pub objects: HashMap<String, SceneObject>,
    pub camera: CameraSettings,
    pub lighting: LightingSettings,
    pub skybox: SkyboxType,
    pub physics: PhysicsSettings,
}

impl Scene {
    pub fn new(name: &str, mode: SceneMode) -> Self;
    pub fn add_object(&mut self, object: SceneObject);
    pub fn remove_object(&mut self, id: &str) -> Option<SceneObject>;
    pub fn get_object(&self, id: &str) -> Option<&SceneObject>;
    pub fn get_object_mut(&mut self, id: &str) -> Option<&mut SceneObject>;
    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>>;
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>>;
}
```

### SceneObject
```rust
pub struct SceneObject {
    pub id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub visible: bool,
    pub children: Vec<String>,
}

impl SceneObject {
    pub fn new_cube(name: String, position: Vec3, size: f32) -> Self;
    pub fn new_sphere(name: String, position: Vec3, radius: f32) -> Self;
    pub fn new_plane(name: String, position: Vec3, width: f32, height: f32) -> Self;
    pub fn new_sprite(name: String, position: Vec3, texture: String, width: f32, height: f32) -> Self;
}
```

### Transform
```rust
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3,  // Euler angles in degrees
    pub scale: Vec3,
}
```

## Examples 🎮

See the `examples/` directory for complete game demos:
- **platformer_2d.wj**: 2D platformer with physics
- **firstperson_3d.wj**: 3D first-person with mouse look

## Troubleshooting 🔧

### Objects Not Visible
- Check visibility toggle in Properties
- Ensure object is in camera view
- Check Z-ordering for 2D

### Lighting Too Dark
- Increase ambient intensity
- Add more lights
- Check light intensity values

### Physics Not Working
- Ensure physics is enabled
- Check gravity vector
- Verify object has physics component

### Scene Won't Save
- Check file permissions
- Ensure valid file path
- Check for serialization errors in console

## Future Enhancements 🚀

- **Hierarchical Parenting**: Parent-child relationships
- **Prefabs**: Reusable object templates
- **Gizmos**: Visual transform manipulators
- **Physics Preview**: Real-time physics simulation
- **Asset Browser**: Drag-and-drop textures/models
- **Undo/Redo**: Multi-level history
- **Multi-Scene**: Load multiple scenes
- **Scene Templates**: Pre-built scene types

---

**Version**: 0.34.0  
**Last Updated**: November 15, 2025  
**Status**: Production Ready ✅

