//! Terrain System
//!
//! Provides heightmap-based terrain with LOD for AAA open-world games.
//!
//! ## Features
//! - Heightmap-based terrain
//! - Level of Detail (LOD) support
//! - Texture splatting (multiple layers)
//! - Normal map generation
//! - Collision mesh generation
//! - Terrain editing tools

use crate::math::{Vec2, Vec3};

/// Terrain heightmap
#[derive(Debug, Clone)]
pub struct Terrain {
    /// Width in vertices
    pub width: usize,
    /// Height in vertices
    pub height: usize,
    /// Height data (row-major)
    heights: Vec<f32>,
    /// World scale (size of each quad)
    pub scale: Vec3,
    /// Height scale (vertical exaggeration)
    pub height_scale: f32,
}

/// Terrain layer (for texture splatting)
#[derive(Debug, Clone)]
pub struct TerrainLayer {
    /// Layer name
    pub name: String,
    /// Texture ID
    pub texture_id: u32,
    /// Normal map ID
    pub normal_map_id: Option<u32>,
    /// Tiling scale
    pub tiling: f32,
    /// Blend sharpness
    pub blend_sharpness: f32,
}

/// Terrain LOD configuration
#[derive(Debug, Clone)]
pub struct TerrainLOD {
    /// LOD levels (distances)
    pub lod_distances: Vec<f32>,
    /// Vertices per LOD level
    pub vertices_per_lod: Vec<usize>,
}

/// Terrain patch (for LOD)
#[derive(Debug, Clone)]
pub struct TerrainPatch {
    /// Patch position (in grid coordinates)
    pub x: usize,
    pub y: usize,
    /// Patch size (in vertices)
    pub size: usize,
    /// Current LOD level
    pub lod_level: usize,
}

impl Terrain {
    /// Create a new terrain
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            heights: vec![0.0; width * height],
            scale: Vec3::new(1.0, 1.0, 1.0),
            height_scale: 1.0,
        }
    }

    /// Create a flat terrain
    pub fn flat(width: usize, height: usize, height_value: f32) -> Self {
        Self {
            width,
            height,
            heights: vec![height_value; width * height],
            scale: Vec3::new(1.0, 1.0, 1.0),
            height_scale: 1.0,
        }
    }

    /// Set scale
    pub fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// Set height scale
    pub fn with_height_scale(mut self, height_scale: f32) -> Self {
        self.height_scale = height_scale;
        self
    }

    /// Get height at grid position
    pub fn get_height(&self, x: usize, y: usize) -> Option<f32> {
        if x >= self.width || y >= self.height {
            return None;
        }
        Some(self.heights[y * self.width + x])
    }

    /// Set height at grid position
    pub fn set_height(&mut self, x: usize, y: usize, height: f32) {
        if x < self.width && y < self.height {
            self.heights[y * self.width + x] = height;
        }
    }

    /// Get height at world position (with interpolation)
    pub fn get_height_at_world(&self, world_x: f32, world_z: f32) -> f32 {
        let grid_x = world_x / self.scale.x;
        let grid_z = world_z / self.scale.z;

        let x0 = grid_x.floor() as usize;
        let z0 = grid_z.floor() as usize;
        let x1 = (x0 + 1).min(self.width - 1);
        let z1 = (z0 + 1).min(self.height - 1);

        let fx = grid_x - x0 as f32;
        let fz = grid_z - z0 as f32;

        // Bilinear interpolation
        let h00 = self.get_height(x0, z0).unwrap_or(0.0);
        let h10 = self.get_height(x1, z0).unwrap_or(0.0);
        let h01 = self.get_height(x0, z1).unwrap_or(0.0);
        let h11 = self.get_height(x1, z1).unwrap_or(0.0);

        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;

        (h0 * (1.0 - fz) + h1 * fz) * self.height_scale
    }

    /// Get normal at grid position
    pub fn get_normal(&self, x: usize, y: usize) -> Vec3 {
        let h_center = self.get_height(x, y).unwrap_or(0.0);
        let h_right = self.get_height(x + 1, y).unwrap_or(h_center);
        let h_up = self.get_height(x, y + 1).unwrap_or(h_center);

        let tangent_x = Vec3::new(self.scale.x, (h_right - h_center) * self.height_scale, 0.0);
        let tangent_z = Vec3::new(0.0, (h_up - h_center) * self.height_scale, self.scale.z);

        tangent_x.cross(tangent_z).normalize()
    }

    /// Raise terrain at position (brush tool)
    pub fn raise(&mut self, center_x: f32, center_z: f32, radius: f32, strength: f32) {
        let grid_center_x = center_x / self.scale.x;
        let grid_center_z = center_z / self.scale.z;
        let grid_radius = radius / self.scale.x;

        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - grid_center_x;
                let dz = y as f32 - grid_center_z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist < grid_radius {
                    let falloff = 1.0 - (dist / grid_radius);
                    let delta = strength * falloff;
                    if let Some(current_height) = self.get_height(x, y) {
                        self.set_height(x, y, current_height + delta);
                    }
                }
            }
        }
    }

    /// Smooth terrain at position
    pub fn smooth(&mut self, center_x: f32, center_z: f32, radius: f32, strength: f32) {
        let grid_center_x = center_x / self.scale.x;
        let grid_center_z = center_z / self.scale.z;
        let grid_radius = radius / self.scale.x;

        // Calculate average height in radius
        let mut sum = 0.0;
        let mut count = 0;

        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - grid_center_x;
                let dz = y as f32 - grid_center_z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist < grid_radius {
                    if let Some(h) = self.get_height(x, y) {
                        sum += h;
                        count += 1;
                    }
                }
            }
        }

        if count == 0 {
            return;
        }

        let average = sum / count as f32;

        // Blend towards average
        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - grid_center_x;
                let dz = y as f32 - grid_center_z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist < grid_radius {
                    let falloff = 1.0 - (dist / grid_radius);
                    if let Some(current_height) = self.get_height(x, y) {
                        let new_height = current_height + (average - current_height) * strength * falloff;
                        self.set_height(x, y, new_height);
                    }
                }
            }
        }
    }

    /// Flatten terrain at position
    pub fn flatten(&mut self, center_x: f32, center_z: f32, radius: f32, target_height: f32, strength: f32) {
        let grid_center_x = center_x / self.scale.x;
        let grid_center_z = center_z / self.scale.z;
        let grid_radius = radius / self.scale.x;

        for y in 0..self.height {
            for x in 0..self.width {
                let dx = x as f32 - grid_center_x;
                let dz = y as f32 - grid_center_z;
                let dist = (dx * dx + dz * dz).sqrt();

                if dist < grid_radius {
                    let falloff = 1.0 - (dist / grid_radius);
                    if let Some(current_height) = self.get_height(x, y) {
                        let new_height = current_height + (target_height - current_height) * strength * falloff;
                        self.set_height(x, y, new_height);
                    }
                }
            }
        }
    }
}

impl Default for TerrainLOD {
    fn default() -> Self {
        Self {
            lod_distances: vec![50.0, 100.0, 200.0, 400.0],
            vertices_per_lod: vec![64, 32, 16, 8],
        }
    }
}

impl TerrainLOD {
    /// Get LOD level for distance
    pub fn get_lod_level(&self, distance: f32) -> usize {
        for (i, &lod_dist) in self.lod_distances.iter().enumerate() {
            if distance < lod_dist {
                return i;
            }
        }
        self.lod_distances.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terrain_creation() {
        let terrain = Terrain::new(10, 10);
        assert_eq!(terrain.width, 10);
        assert_eq!(terrain.height, 10);
        println!("✅ Terrain created");
    }

    #[test]
    fn test_flat_terrain() {
        let terrain = Terrain::flat(10, 10, 5.0);
        assert_eq!(terrain.get_height(5, 5), Some(5.0));
        println!("✅ Flat terrain");
    }

    #[test]
    fn test_set_get_height() {
        let mut terrain = Terrain::new(10, 10);
        terrain.set_height(5, 5, 10.0);
        assert_eq!(terrain.get_height(5, 5), Some(10.0));
        println!("✅ Set/get height");
    }

    #[test]
    fn test_world_height_interpolation() {
        let mut terrain = Terrain::new(10, 10);
        terrain.set_height(0, 0, 0.0);
        terrain.set_height(1, 0, 10.0);
        terrain.set_height(0, 1, 0.0);
        terrain.set_height(1, 1, 10.0);

        // Midpoint should be 5.0
        let h = terrain.get_height_at_world(0.5, 0.5);
        assert!((h - 5.0).abs() < 0.1);
        println!("✅ World height interpolation: {}", h);
    }

    #[test]
    fn test_terrain_scale() {
        let terrain = Terrain::new(10, 10).with_scale(Vec3::new(2.0, 1.0, 2.0));
        assert_eq!(terrain.scale, Vec3::new(2.0, 1.0, 2.0));
        println!("✅ Terrain scale");
    }

    #[test]
    fn test_height_scale() {
        let terrain = Terrain::new(10, 10).with_height_scale(2.0);
        assert_eq!(terrain.height_scale, 2.0);
        println!("✅ Height scale");
    }

    #[test]
    fn test_raise_terrain() {
        let mut terrain = Terrain::flat(10, 10, 0.0);
        terrain.raise(5.0, 5.0, 2.0, 1.0);

        let center_height = terrain.get_height(5, 5).unwrap();
        assert!(center_height > 0.0);
        println!("✅ Raise terrain: center height = {}", center_height);
    }

    #[test]
    fn test_smooth_terrain() {
        let mut terrain = Terrain::new(10, 10);
        terrain.set_height(5, 5, 10.0);
        terrain.set_height(5, 6, 0.0);

        terrain.smooth(5.0, 5.5, 2.0, 1.0);

        let h1 = terrain.get_height(5, 5).unwrap();
        let h2 = terrain.get_height(5, 6).unwrap();

        // Heights should be closer together
        assert!((h1 - h2).abs() < 10.0);
        println!("✅ Smooth terrain: {} -> {}", h1, h2);
    }

    #[test]
    fn test_flatten_terrain() {
        let mut terrain = Terrain::new(10, 10);
        terrain.set_height(5, 5, 10.0);

        terrain.flatten(5.0, 5.0, 2.0, 5.0, 1.0);

        let h = terrain.get_height(5, 5).unwrap();
        assert!((h - 5.0).abs() < 1.0);
        println!("✅ Flatten terrain: {}", h);
    }

    #[test]
    fn test_terrain_normal() {
        let mut terrain = Terrain::new(10, 10);
        terrain.set_height(5, 5, 0.0);
        terrain.set_height(6, 5, 1.0);
        terrain.set_height(5, 6, 1.0);

        let normal = terrain.get_normal(5, 5);
        assert!(normal.length() > 0.9 && normal.length() < 1.1); // Should be normalized
        println!("✅ Terrain normal: {:?}", normal);
    }

    #[test]
    fn test_lod_default() {
        let lod = TerrainLOD::default();
        assert_eq!(lod.lod_distances.len(), 4);
        assert_eq!(lod.vertices_per_lod.len(), 4);
        println!("✅ TerrainLOD default");
    }

    #[test]
    fn test_lod_level_selection() {
        let lod = TerrainLOD::default();
        assert_eq!(lod.get_lod_level(25.0), 0);
        assert_eq!(lod.get_lod_level(75.0), 1);
        assert_eq!(lod.get_lod_level(150.0), 2);
        assert_eq!(lod.get_lod_level(300.0), 3);
        assert_eq!(lod.get_lod_level(500.0), 4);
        println!("✅ LOD level selection");
    }
}

