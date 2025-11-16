//! Physically-Based Rendering (PBR) System
//!
//! Provides PBR material and lighting calculations for realistic rendering.
//!
//! ## Features
//! - PBR material properties (metallic-roughness workflow)
//! - Multiple light types (directional, point, spot)
//! - Image-based lighting (IBL)
//! - Shadow mapping support
//! - Normal mapping
//! - Emissive materials

use crate::math::{Vec3, Vec4};

/// PBR material properties
#[derive(Debug, Clone)]
pub struct PBRMaterial {
    /// Base color (albedo)
    pub base_color: Vec4,
    /// Metallic factor (0.0 = dielectric, 1.0 = metal)
    pub metallic: f32,
    /// Roughness factor (0.0 = smooth, 1.0 = rough)
    pub roughness: f32,
    /// Emissive color
    pub emissive: Vec3,
    /// Emissive strength
    pub emissive_strength: f32,
    /// Normal map strength
    pub normal_strength: f32,
    /// Occlusion strength
    pub occlusion_strength: f32,
    /// Alpha cutoff for alpha testing
    pub alpha_cutoff: f32,
    /// Alpha mode
    pub alpha_mode: AlphaMode,
    /// Base color texture (optional)
    pub base_color_texture: Option<TextureHandle>,
    /// Metallic-roughness texture (optional, B=metallic, G=roughness)
    pub metallic_roughness_texture: Option<TextureHandle>,
    /// Normal map texture (optional)
    pub normal_texture: Option<TextureHandle>,
    /// Ambient occlusion texture (optional)
    pub occlusion_texture: Option<TextureHandle>,
    /// Emissive texture (optional)
    pub emissive_texture: Option<TextureHandle>,
}

/// Handle to a texture resource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

/// Alpha blending mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaMode {
    /// Fully opaque
    Opaque,
    /// Alpha testing with cutoff
    Mask,
    /// Alpha blending
    Blend,
}

/// Light type
#[derive(Debug, Clone)]
pub enum Light {
    /// Directional light (sun)
    Directional(DirectionalLight),
    /// Point light (bulb)
    Point(PointLight),
    /// Spot light (flashlight)
    Spot(SpotLight),
}

/// Directional light (parallel rays, like sun)
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    /// Light direction (normalized)
    pub direction: Vec3,
    /// Light color
    pub color: Vec3,
    /// Light intensity
    pub intensity: f32,
    /// Cast shadows
    pub cast_shadows: bool,
}

/// Point light (radiates in all directions)
#[derive(Debug, Clone)]
pub struct PointLight {
    /// Light position
    pub position: Vec3,
    /// Light color
    pub color: Vec3,
    /// Light intensity
    pub intensity: f32,
    /// Attenuation range
    pub range: f32,
    /// Cast shadows
    pub cast_shadows: bool,
}

/// Spot light (cone of light)
#[derive(Debug, Clone)]
pub struct SpotLight {
    /// Light position
    pub position: Vec3,
    /// Light direction (normalized)
    pub direction: Vec3,
    /// Light color
    pub color: Vec3,
    /// Light intensity
    pub intensity: f32,
    /// Inner cone angle (radians)
    pub inner_angle: f32,
    /// Outer cone angle (radians)
    pub outer_angle: f32,
    /// Attenuation range
    pub range: f32,
    /// Cast shadows
    pub cast_shadows: bool,
}

/// Environment map for image-based lighting
#[derive(Debug, Clone)]
pub struct EnvironmentMap {
    /// Diffuse irradiance map ID
    pub irradiance_map: u32,
    /// Specular radiance map ID
    pub radiance_map: u32,
    /// BRDF lookup table ID
    pub brdf_lut: u32,
    /// Environment intensity
    pub intensity: f32,
}

/// Shadow map configuration
#[derive(Debug, Clone)]
pub struct ShadowMap {
    /// Shadow map resolution
    pub resolution: u32,
    /// Shadow bias
    pub bias: f32,
    /// Shadow normal bias
    pub normal_bias: f32,
    /// PCF filter size
    pub filter_size: u32,
}

impl Default for PBRMaterial {
    fn default() -> Self {
        Self {
            base_color: Vec4::new(1.0, 1.0, 1.0, 1.0),
            metallic: 0.0,
            roughness: 0.5,
            emissive: Vec3::new(0.0, 0.0, 0.0),
            emissive_strength: 0.0,
            normal_strength: 1.0,
            occlusion_strength: 1.0,
            alpha_cutoff: 0.5,
            alpha_mode: AlphaMode::Opaque,
            base_color_texture: None,
            metallic_roughness_texture: None,
            normal_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
        }
    }
}

impl PBRMaterial {
    /// Create a new PBR material
    pub fn new() -> Self {
        Self::default()
    }

    /// Set base color
    pub fn with_color(mut self, color: Vec4) -> Self {
        self.base_color = color;
        self
    }

    /// Set metallic factor
    pub fn with_metallic(mut self, metallic: f32) -> Self {
        self.metallic = metallic.clamp(0.0, 1.0);
        self
    }

    /// Set roughness factor
    pub fn with_roughness(mut self, roughness: f32) -> Self {
        self.roughness = roughness.clamp(0.0, 1.0);
        self
    }

    /// Set emissive color
    pub fn with_emissive(mut self, emissive: Vec3, strength: f32) -> Self {
        self.emissive = emissive;
        self.emissive_strength = strength;
        self
    }

    /// Set alpha mode
    pub fn with_alpha_mode(mut self, mode: AlphaMode) -> Self {
        self.alpha_mode = mode;
        self
    }

    /// Set base color texture
    pub fn with_base_color_texture(mut self, texture: TextureHandle) -> Self {
        self.base_color_texture = Some(texture);
        self
    }

    /// Set metallic-roughness texture
    pub fn with_metallic_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.metallic_roughness_texture = Some(texture);
        self
    }

    /// Set normal map texture
    pub fn with_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.normal_texture = Some(texture);
        self
    }

    /// Set ambient occlusion texture
    pub fn with_occlusion_texture(mut self, texture: TextureHandle) -> Self {
        self.occlusion_texture = Some(texture);
        self
    }

    /// Set emissive texture
    pub fn with_emissive_texture(mut self, texture: TextureHandle) -> Self {
        self.emissive_texture = Some(texture);
        self
    }

    /// Create a metallic material
    pub fn metallic(color: Vec3, roughness: f32) -> Self {
        Self {
            base_color: color.extend(1.0),
            metallic: 1.0,
            roughness: roughness.clamp(0.0, 1.0),
            ..Default::default()
        }
    }

    /// Create a dielectric material
    pub fn dielectric(color: Vec3, roughness: f32) -> Self {
        Self {
            base_color: color.extend(1.0),
            metallic: 0.0,
            roughness: roughness.clamp(0.0, 1.0),
            ..Default::default()
        }
    }
}

impl DirectionalLight {
    /// Create a new directional light
    pub fn new(direction: Vec3, color: Vec3, intensity: f32) -> Self {
        Self {
            direction: direction.normalize(),
            color,
            intensity,
            cast_shadows: true,
        }
    }

    /// Create a sun light
    pub fn sun() -> Self {
        Self::new(
            Vec3::new(-0.3, -1.0, -0.3),
            Vec3::new(1.0, 0.95, 0.9),
            1.0,
        )
    }
}

impl PointLight {
    /// Create a new point light
    pub fn new(position: Vec3, color: Vec3, intensity: f32, range: f32) -> Self {
        Self {
            position,
            color,
            intensity,
            range,
            cast_shadows: false,
        }
    }
}

impl SpotLight {
    /// Create a new spot light
    pub fn new(
        position: Vec3,
        direction: Vec3,
        color: Vec3,
        intensity: f32,
        inner_angle: f32,
        outer_angle: f32,
        range: f32,
    ) -> Self {
        Self {
            position,
            direction: direction.normalize(),
            color,
            intensity,
            inner_angle,
            outer_angle,
            range,
            cast_shadows: false,
        }
    }

    /// Create a flashlight
    pub fn flashlight(position: Vec3, direction: Vec3) -> Self {
        Self::new(
            position,
            direction,
            Vec3::new(1.0, 1.0, 0.9),
            10.0,
            0.3,
            0.5,
            20.0,
        )
    }
}

impl Default for ShadowMap {
    fn default() -> Self {
        Self {
            resolution: 2048,
            bias: 0.005,
            normal_bias: 0.01,
            filter_size: 2,
        }
    }
}

impl EnvironmentMap {
    /// Create a new environment map
    pub fn new(irradiance_map: u32, radiance_map: u32, brdf_lut: u32) -> Self {
        Self {
            irradiance_map,
            radiance_map,
            brdf_lut,
            intensity: 1.0,
        }
    }

    /// Set intensity
    pub fn with_intensity(mut self, intensity: f32) -> Self {
        self.intensity = intensity;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pbr_material_default() {
        let mat = PBRMaterial::default();
        assert_eq!(mat.base_color, Vec4::new(1.0, 1.0, 1.0, 1.0));
        assert_eq!(mat.metallic, 0.0);
        assert_eq!(mat.roughness, 0.5);
        assert_eq!(mat.alpha_mode, AlphaMode::Opaque);
        println!("✅ PBRMaterial default");
    }

    #[test]
    fn test_pbr_material_builder() {
        let mat = PBRMaterial::new()
            .with_color(Vec4::new(1.0, 0.0, 0.0, 1.0))
            .with_metallic(0.8)
            .with_roughness(0.3);

        assert_eq!(mat.base_color, Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(mat.metallic, 0.8);
        assert_eq!(mat.roughness, 0.3);
        println!("✅ PBRMaterial builder");
    }

    #[test]
    fn test_metallic_material() {
        let mat = PBRMaterial::metallic(Vec3::new(0.8, 0.8, 0.8), 0.2);
        assert_eq!(mat.metallic, 1.0);
        assert_eq!(mat.roughness, 0.2);
        println!("✅ Metallic material");
    }

    #[test]
    fn test_dielectric_material() {
        let mat = PBRMaterial::dielectric(Vec3::new(1.0, 0.0, 0.0), 0.4);
        assert_eq!(mat.metallic, 0.0);
        assert_eq!(mat.roughness, 0.4);
        println!("✅ Dielectric material");
    }

    #[test]
    fn test_directional_light() {
        let light = DirectionalLight::new(
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            1.0,
        );
        assert_eq!(light.direction.length(), 1.0); // Normalized
        assert_eq!(light.intensity, 1.0);
        println!("✅ DirectionalLight");
    }

    #[test]
    fn test_sun_light() {
        let sun = DirectionalLight::sun();
        assert_eq!(sun.direction.length(), 1.0);
        assert!(sun.cast_shadows);
        println!("✅ Sun light preset");
    }

    #[test]
    fn test_point_light() {
        let light = PointLight::new(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            10.0,
            20.0,
        );
        assert_eq!(light.position, Vec3::new(0.0, 5.0, 0.0));
        assert_eq!(light.intensity, 10.0);
        assert_eq!(light.range, 20.0);
        println!("✅ PointLight");
    }

    #[test]
    fn test_spot_light() {
        let light = SpotLight::new(
            Vec3::new(0.0, 5.0, 0.0),
            Vec3::new(0.0, -1.0, 0.0),
            Vec3::new(1.0, 1.0, 1.0),
            10.0,
            0.3,
            0.5,
            20.0,
        );
        assert_eq!(light.direction.length(), 1.0); // Normalized
        assert_eq!(light.inner_angle, 0.3);
        assert_eq!(light.outer_angle, 0.5);
        println!("✅ SpotLight");
    }

    #[test]
    fn test_flashlight_preset() {
        let flashlight = SpotLight::flashlight(
            Vec3::new(0.0, 1.0, 0.0),
            Vec3::new(0.0, 0.0, -1.0),
        );
        assert_eq!(flashlight.direction.length(), 1.0);
        assert!(flashlight.intensity > 0.0);
        println!("✅ Flashlight preset");
    }

    #[test]
    fn test_shadow_map_default() {
        let shadow = ShadowMap::default();
        assert_eq!(shadow.resolution, 2048);
        assert_eq!(shadow.bias, 0.005);
        assert_eq!(shadow.filter_size, 2);
        println!("✅ ShadowMap default");
    }

    #[test]
    fn test_environment_map() {
        let env = EnvironmentMap::new(1, 2, 3);
        assert_eq!(env.irradiance_map, 1);
        assert_eq!(env.radiance_map, 2);
        assert_eq!(env.brdf_lut, 3);
        assert_eq!(env.intensity, 1.0);
        println!("✅ EnvironmentMap");
    }

    #[test]
    fn test_environment_map_intensity() {
        let env = EnvironmentMap::new(1, 2, 3).with_intensity(0.5);
        assert_eq!(env.intensity, 0.5);
        println!("✅ EnvironmentMap intensity");
    }

    #[test]
    fn test_alpha_modes() {
        let opaque = PBRMaterial::default();
        assert_eq!(opaque.alpha_mode, AlphaMode::Opaque);

        let mask = PBRMaterial::default().with_alpha_mode(AlphaMode::Mask);
        assert_eq!(mask.alpha_mode, AlphaMode::Mask);

        let blend = PBRMaterial::default().with_alpha_mode(AlphaMode::Blend);
        assert_eq!(blend.alpha_mode, AlphaMode::Blend);

        println!("✅ Alpha modes");
    }

    #[test]
    fn test_emissive_material() {
        let mat = PBRMaterial::new()
            .with_emissive(Vec3::new(1.0, 0.5, 0.0), 2.0);
        assert_eq!(mat.emissive, Vec3::new(1.0, 0.5, 0.0));
        assert_eq!(mat.emissive_strength, 2.0);
        println!("✅ Emissive material");
    }

    #[test]
    fn test_roughness_clamping() {
        let mat1 = PBRMaterial::new().with_roughness(-0.5);
        assert_eq!(mat1.roughness, 0.0);

        let mat2 = PBRMaterial::new().with_roughness(1.5);
        assert_eq!(mat2.roughness, 1.0);

        println!("✅ Roughness clamping");
    }

    #[test]
    fn test_metallic_clamping() {
        let mat1 = PBRMaterial::new().with_metallic(-0.5);
        assert_eq!(mat1.metallic, 0.0);

        let mat2 = PBRMaterial::new().with_metallic(1.5);
        assert_eq!(mat2.metallic, 1.0);

        println!("✅ Metallic clamping");
    }
}

