/// Scene management for Windjammer Game Editor
/// Supports 2D and 3D games with greybox primitives, lighting, and skybox
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use serde::{Deserialize, Serialize};
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
use std::collections::HashMap;

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub name: String,
    pub objects: HashMap<String, SceneObject>,
    pub camera: Camera,
    pub lighting: Lighting,
    pub skybox: Skybox,
    pub physics: PhysicsSettings,
    pub mode: SceneMode,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum SceneMode {
    TwoD,
    ThreeD,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub visible: bool,
    pub children: Vec<String>, // IDs of child objects
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectType {
    // 3D Primitives (Greybox)
    Cube {
        size: f32,
    },
    Sphere {
        radius: f32,
    },
    Plane {
        width: f32,
        height: f32,
    },
    Cylinder {
        radius: f32,
        height: f32,
    },
    Capsule {
        radius: f32,
        height: f32,
    },

    // 2D Objects
    Sprite {
        texture: String,
        width: f32,
        height: f32,
    },
    TileMap {
        tiles: Vec<Vec<u32>>,
        tile_size: f32,
    },

    // Lights
    DirectionalLight {
        color: Color,
        intensity: f32,
    },
    PointLight {
        color: Color,
        intensity: f32,
        range: f32,
    },
    SpotLight {
        color: Color,
        intensity: f32,
        range: f32,
        angle: f32,
    },

    // Special
    Camera,
    Empty, // Container for grouping
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Transform {
    pub position: Vec3,
    pub rotation: Vec3, // Euler angles in degrees
    pub scale: Vec3,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Vec3,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
    pub orthographic: bool,
    pub ortho_size: f32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lighting {
    pub ambient_color: Color,
    pub ambient_intensity: f32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Skybox {
    SolidColor(Color),
    Gradient { top: Color, bottom: Color },
    Cubemap { path: String },
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsSettings {
    pub enabled: bool,
    pub gravity: Vec3,
}

// Default implementations
#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for Scene {
    fn default() -> Self {
        let mut objects = HashMap::new();

        // Add default camera
        objects.insert(
            "Camera".to_string(),
            SceneObject {
                id: "Camera".to_string(),
                name: "Main Camera".to_string(),
                object_type: ObjectType::Camera,
                transform: Transform {
                    position: Vec3 {
                        x: 0.0,
                        y: 2.0,
                        z: 10.0,
                    },
                    rotation: Vec3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    scale: Vec3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                },
                visible: true,
                children: vec![],
            },
        );

        // Add default directional light
        objects.insert(
            "DirectionalLight".to_string(),
            SceneObject {
                id: "DirectionalLight".to_string(),
                name: "Sun".to_string(),
                object_type: ObjectType::DirectionalLight {
                    color: Color {
                        r: 1.0,
                        g: 1.0,
                        b: 0.9,
                        a: 1.0,
                    },
                    intensity: 1.0,
                },
                transform: Transform {
                    position: Vec3 {
                        x: 0.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    rotation: Vec3 {
                        x: -45.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    scale: Vec3 {
                        x: 1.0,
                        y: 1.0,
                        z: 1.0,
                    },
                },
                visible: true,
                children: vec![],
            },
        );

        Self {
            name: "New Scene".to_string(),
            objects,
            camera: Camera::default(),
            lighting: Lighting::default(),
            skybox: Skybox::default(),
            physics: PhysicsSettings::default(),
            mode: SceneMode::ThreeD,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Vec3 {
                x: 0.0,
                y: 2.0,
                z: 10.0,
            },
            rotation: Vec3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            fov: 60.0,
            near: 0.1,
            far: 1000.0,
            orthographic: false,
            ortho_size: 10.0,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for Lighting {
    fn default() -> Self {
        Self {
            ambient_color: Color {
                r: 0.3,
                g: 0.3,
                b: 0.4,
                a: 1.0,
            },
            ambient_intensity: 0.2,
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for Skybox {
    fn default() -> Self {
        Self::Gradient {
            top: Color {
                r: 0.5,
                g: 0.7,
                b: 1.0,
                a: 1.0,
            },
            bottom: Color {
                r: 0.8,
                g: 0.9,
                b: 1.0,
                a: 1.0,
            },
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Default for PhysicsSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            gravity: Vec3 {
                x: 0.0,
                y: -9.81,
                z: 0.0,
            },
        }
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl Scene {
    pub fn new(name: String, mode: SceneMode) -> Self {
        let mut scene = Self::default();
        scene.name = name;
        scene.mode = mode;

        // Adjust defaults for 2D mode
        if mode == SceneMode::TwoD {
            scene.camera.orthographic = true;
            scene.camera.position = Vec3 {
                x: 0.0,
                y: 0.0,
                z: 10.0,
            };
            scene.physics.gravity = Vec3 {
                x: 0.0,
                y: -9.81,
                z: 0.0,
            };
        }

        scene
    }

    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.insert(object.id.clone(), object);
    }

    pub fn remove_object(&mut self, id: &str) -> Option<SceneObject> {
        self.objects.remove(id)
    }

    pub fn get_object(&self, id: &str) -> Option<&SceneObject> {
        self.objects.get(id)
    }

    pub fn get_object_mut(&mut self, id: &str) -> Option<&mut SceneObject> {
        self.objects.get_mut(id)
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let scene = serde_json::from_str(&json)?;
        Ok(scene)
    }
}

#[cfg(all(not(target_arch = "wasm32"), feature = "desktop"))]
impl SceneObject {
    pub fn new_cube(name: String, position: Vec3, size: f32) -> Self {
        Self {
            id: format!("Cube_{}", uuid::Uuid::new_v4()),
            name,
            object_type: ObjectType::Cube { size },
            transform: Transform {
                position,
                rotation: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                scale: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            },
            visible: true,
            children: vec![],
        }
    }

    pub fn new_sphere(name: String, position: Vec3, radius: f32) -> Self {
        Self {
            id: format!("Sphere_{}", uuid::Uuid::new_v4()),
            name,
            object_type: ObjectType::Sphere { radius },
            transform: Transform {
                position,
                rotation: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                scale: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            },
            visible: true,
            children: vec![],
        }
    }

    pub fn new_plane(name: String, position: Vec3, width: f32, height: f32) -> Self {
        Self {
            id: format!("Plane_{}", uuid::Uuid::new_v4()),
            name,
            object_type: ObjectType::Plane { width, height },
            transform: Transform {
                position,
                rotation: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                scale: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            },
            visible: true,
            children: vec![],
        }
    }

    pub fn new_sprite(
        name: String,
        position: Vec3,
        texture: String,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            id: format!("Sprite_{}", uuid::Uuid::new_v4()),
            name,
            object_type: ObjectType::Sprite {
                texture,
                width,
                height,
            },
            transform: Transform {
                position,
                rotation: Vec3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                scale: Vec3 {
                    x: 1.0,
                    y: 1.0,
                    z: 1.0,
                },
            },
            visible: true,
            children: vec![],
        }
    }
}
