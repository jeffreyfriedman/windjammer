//! Post-Processing Effects System
//!
//! Provides AAA-quality post-processing effects for enhanced visuals.
//!
//! ## Features
//! - Bloom (HDR glow)
//! - Depth of Field (DOF)
//! - Motion Blur
//! - Tone Mapping
//! - Color Grading
//! - Vignette
//! - Chromatic Aberration
//! - Film Grain
//! - SSAO (Screen-Space Ambient Occlusion)
//! - HDR (High Dynamic Range)

use crate::math::{Vec2, Vec3};
use std::sync::Arc;

/// Post-processing stack
#[derive(Debug, Clone)]
pub struct PostProcessing {
    /// Bloom effect
    pub bloom: Option<BloomEffect>,
    /// Depth of field
    pub depth_of_field: Option<DepthOfFieldEffect>,
    /// Motion blur
    pub motion_blur: Option<MotionBlurEffect>,
    /// Tone mapping
    pub tone_mapping: ToneMappingMode,
    /// Color grading
    pub color_grading: Option<ColorGrading>,
    /// Vignette
    pub vignette: Option<VignetteEffect>,
    /// Chromatic aberration
    pub chromatic_aberration: Option<ChromaticAberrationEffect>,
    /// Film grain
    pub film_grain: Option<FilmGrainEffect>,
    /// SSAO (Screen-Space Ambient Occlusion)
    pub ssao: Option<SSAOEffect>,
    /// HDR settings
    pub hdr: HDRSettings,
    /// Enabled
    pub enabled: bool,
}

/// Bloom effect (HDR glow)
#[derive(Debug, Clone)]
pub struct BloomEffect {
    /// Bloom threshold (brightness threshold)
    pub threshold: f32,
    /// Bloom intensity
    pub intensity: f32,
    /// Number of blur passes
    pub blur_passes: u32,
    /// Blur radius
    pub blur_radius: f32,
}

/// Depth of field effect
#[derive(Debug, Clone)]
pub struct DepthOfFieldEffect {
    /// Focus distance
    pub focus_distance: f32,
    /// Focus range
    pub focus_range: f32,
    /// Bokeh intensity
    pub bokeh_intensity: f32,
    /// Aperture size
    pub aperture: f32,
}

/// Motion blur effect
#[derive(Debug, Clone)]
pub struct MotionBlurEffect {
    /// Blur intensity
    pub intensity: f32,
    /// Number of samples
    pub samples: u32,
    /// Shutter angle (degrees)
    pub shutter_angle: f32,
}

/// Tone mapping mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneMappingMode {
    /// No tone mapping
    None,
    /// Reinhard tone mapping
    Reinhard,
    /// ACES filmic tone mapping
    ACES,
    /// Uncharted 2 tone mapping
    Uncharted2,
}

/// Color grading
#[derive(Debug, Clone)]
pub struct ColorGrading {
    /// Exposure adjustment
    pub exposure: f32,
    /// Contrast adjustment
    pub contrast: f32,
    /// Saturation adjustment
    pub saturation: f32,
    /// Temperature (warm/cool)
    pub temperature: f32,
    /// Tint (green/magenta)
    pub tint: f32,
    /// Color filter
    pub color_filter: Vec3,
}

/// Vignette effect
#[derive(Debug, Clone)]
pub struct VignetteEffect {
    /// Vignette intensity
    pub intensity: f32,
    /// Vignette smoothness
    pub smoothness: f32,
    /// Vignette roundness
    pub roundness: f32,
}

/// Chromatic aberration effect
#[derive(Debug, Clone)]
pub struct ChromaticAberrationEffect {
    /// Aberration intensity
    pub intensity: f32,
    /// Aberration offset
    pub offset: Vec2,
}

/// Film grain effect
#[derive(Debug, Clone)]
pub struct FilmGrainEffect {
    /// Grain intensity
    pub intensity: f32,
    /// Grain size
    pub size: f32,
}

/// SSAO (Screen-Space Ambient Occlusion) effect
#[derive(Debug, Clone)]
pub struct SSAOEffect {
    /// SSAO radius (sampling radius)
    pub radius: f32,
    /// SSAO bias (to prevent self-shadowing)
    pub bias: f32,
    /// Number of samples
    pub samples: u32,
    /// SSAO intensity
    pub intensity: f32,
    /// Blur radius for SSAO blur pass
    pub blur_radius: f32,
}

impl Default for SSAOEffect {
    fn default() -> Self {
        Self {
            radius: 0.5,
            bias: 0.025,
            samples: 16,
            intensity: 1.0,
            blur_radius: 2.0,
        }
    }
}

impl SSAOEffect {
    /// Create a high-quality SSAO effect
    pub fn high_quality() -> Self {
        Self {
            radius: 0.5,
            bias: 0.025,
            samples: 32,
            intensity: 1.2,
            blur_radius: 2.0,
        }
    }

    /// Create a performance-friendly SSAO effect
    pub fn performance() -> Self {
        Self {
            radius: 0.4,
            bias: 0.03,
            samples: 8,
            intensity: 0.8,
            blur_radius: 1.5,
        }
    }
}

/// HDR (High Dynamic Range) settings
#[derive(Debug, Clone)]
pub struct HDRSettings {
    /// Enable HDR
    pub enabled: bool,
    /// Exposure value
    pub exposure: f32,
    /// White point (for tone mapping)
    pub white_point: f32,
    /// Adaptation speed (for auto-exposure)
    pub adaptation_speed: f32,
}

impl Default for HDRSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            exposure: 1.0,
            white_point: 11.2,
            adaptation_speed: 1.0,
        }
    }
}

impl HDRSettings {
    /// Create HDR settings for bright outdoor scenes
    pub fn outdoor() -> Self {
        Self {
            enabled: true,
            exposure: 0.8,
            white_point: 11.2,
            adaptation_speed: 0.5,
        }
    }

    /// Create HDR settings for dark indoor scenes
    pub fn indoor() -> Self {
        Self {
            enabled: true,
            exposure: 1.5,
            white_point: 11.2,
            adaptation_speed: 1.5,
        }
    }
}

impl Default for PostProcessing {
    fn default() -> Self {
        Self {
            bloom: None,
            depth_of_field: None,
            motion_blur: None,
            tone_mapping: ToneMappingMode::ACES,
            color_grading: None,
            vignette: None,
            chromatic_aberration: None,
            film_grain: None,
            ssao: None,
            hdr: HDRSettings::default(),
            enabled: true,
        }
    }
}

impl PostProcessing {
    /// Create a new post-processing stack
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable bloom
    pub fn with_bloom(mut self, bloom: BloomEffect) -> Self {
        self.bloom = Some(bloom);
        self
    }

    /// Enable depth of field
    pub fn with_dof(mut self, dof: DepthOfFieldEffect) -> Self {
        self.depth_of_field = Some(dof);
        self
    }

    /// Enable motion blur
    pub fn with_motion_blur(mut self, motion_blur: MotionBlurEffect) -> Self {
        self.motion_blur = Some(motion_blur);
        self
    }

    /// Set tone mapping
    pub fn with_tone_mapping(mut self, mode: ToneMappingMode) -> Self {
        self.tone_mapping = mode;
        self
    }

    /// Enable color grading
    pub fn with_color_grading(mut self, grading: ColorGrading) -> Self {
        self.color_grading = Some(grading);
        self
    }

    /// Enable vignette
    pub fn with_vignette(mut self, vignette: VignetteEffect) -> Self {
        self.vignette = Some(vignette);
        self
    }

    /// Enable SSAO
    pub fn with_ssao(mut self, ssao: SSAOEffect) -> Self {
        self.ssao = Some(ssao);
        self
    }

    /// Set HDR settings
    pub fn with_hdr(mut self, hdr: HDRSettings) -> Self {
        self.hdr = hdr;
        self
    }

    /// Create a cinematic preset
    pub fn cinematic() -> Self {
        Self::new()
            .with_bloom(BloomEffect::default())
            .with_dof(DepthOfFieldEffect::cinematic())
            .with_color_grading(ColorGrading::cinematic())
            .with_vignette(VignetteEffect::default())
            .with_ssao(SSAOEffect::default())
            .with_hdr(HDRSettings::default())
            .with_tone_mapping(ToneMappingMode::ACES)
    }

    /// Create a stylized preset
    pub fn stylized() -> Self {
        Self::new()
            .with_bloom(BloomEffect::strong())
            .with_color_grading(ColorGrading::vibrant())
            .with_vignette(VignetteEffect::subtle())
            .with_tone_mapping(ToneMappingMode::Reinhard)
    }
}

impl Default for BloomEffect {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            intensity: 0.5,
            blur_passes: 5,
            blur_radius: 1.0,
        }
    }
}

impl BloomEffect {
    /// Create a strong bloom effect
    pub fn strong() -> Self {
        Self {
            threshold: 0.8,
            intensity: 1.0,
            blur_passes: 7,
            blur_radius: 1.5,
        }
    }

    /// Create a subtle bloom effect
    pub fn subtle() -> Self {
        Self {
            threshold: 1.2,
            intensity: 0.3,
            blur_passes: 3,
            blur_radius: 0.8,
        }
    }
}

impl Default for DepthOfFieldEffect {
    fn default() -> Self {
        Self {
            focus_distance: 10.0,
            focus_range: 5.0,
            bokeh_intensity: 1.0,
            aperture: 2.8,
        }
    }
}

impl DepthOfFieldEffect {
    /// Create a cinematic DOF
    pub fn cinematic() -> Self {
        Self {
            focus_distance: 10.0,
            focus_range: 3.0,
            bokeh_intensity: 1.5,
            aperture: 1.8,
        }
    }
}

impl Default for MotionBlurEffect {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            samples: 8,
            shutter_angle: 180.0,
        }
    }
}

impl Default for ColorGrading {
    fn default() -> Self {
        Self {
            exposure: 0.0,
            contrast: 1.0,
            saturation: 1.0,
            temperature: 0.0,
            tint: 0.0,
            color_filter: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl ColorGrading {
    /// Create a cinematic color grading
    pub fn cinematic() -> Self {
        Self {
            exposure: 0.2,
            contrast: 1.1,
            saturation: 0.9,
            temperature: -0.1,
            tint: 0.05,
            color_filter: Vec3::new(1.0, 0.98, 0.95),
        }
    }

    /// Create a vibrant color grading
    pub fn vibrant() -> Self {
        Self {
            exposure: 0.1,
            contrast: 1.2,
            saturation: 1.3,
            temperature: 0.0,
            tint: 0.0,
            color_filter: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Default for VignetteEffect {
    fn default() -> Self {
        Self {
            intensity: 0.3,
            smoothness: 0.5,
            roundness: 1.0,
        }
    }
}

impl VignetteEffect {
    /// Create a subtle vignette
    pub fn subtle() -> Self {
        Self {
            intensity: 0.2,
            smoothness: 0.7,
            roundness: 1.0,
        }
    }
}

impl Default for ChromaticAberrationEffect {
    fn default() -> Self {
        Self {
            intensity: 0.5,
            offset: Vec2::new(0.0, 0.0),
        }
    }
}

impl Default for FilmGrainEffect {
    fn default() -> Self {
        Self {
            intensity: 0.1,
            size: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_processing_creation() {
        let pp = PostProcessing::new();
        assert!(pp.enabled);
        assert_eq!(pp.tone_mapping, ToneMappingMode::ACES);
        println!("✅ PostProcessing created");
    }

    #[test]
    fn test_bloom_default() {
        let bloom = BloomEffect::default();
        assert_eq!(bloom.threshold, 1.0);
        assert_eq!(bloom.intensity, 0.5);
        println!("✅ BloomEffect default");
    }

    #[test]
    fn test_bloom_presets() {
        let strong = BloomEffect::strong();
        let subtle = BloomEffect::subtle();

        assert!(strong.intensity > subtle.intensity);
        assert!(strong.threshold < subtle.threshold);
        println!("✅ Bloom presets");
    }

    #[test]
    fn test_dof_default() {
        let dof = DepthOfFieldEffect::default();
        assert_eq!(dof.focus_distance, 10.0);
        assert_eq!(dof.aperture, 2.8);
        println!("✅ DepthOfFieldEffect default");
    }

    #[test]
    fn test_dof_cinematic() {
        let dof = DepthOfFieldEffect::cinematic();
        assert!(dof.aperture < 2.0); // Wider aperture
        println!("✅ Cinematic DOF");
    }

    #[test]
    fn test_motion_blur_default() {
        let mb = MotionBlurEffect::default();
        assert_eq!(mb.intensity, 0.5);
        assert_eq!(mb.samples, 8);
        println!("✅ MotionBlurEffect default");
    }

    #[test]
    fn test_tone_mapping_modes() {
        assert_eq!(ToneMappingMode::None, ToneMappingMode::None);
        assert_ne!(ToneMappingMode::ACES, ToneMappingMode::Reinhard);
        println!("✅ Tone mapping modes");
    }

    #[test]
    fn test_color_grading_default() {
        let cg = ColorGrading::default();
        assert_eq!(cg.exposure, 0.0);
        assert_eq!(cg.contrast, 1.0);
        assert_eq!(cg.saturation, 1.0);
        println!("✅ ColorGrading default");
    }

    #[test]
    fn test_color_grading_presets() {
        let cinematic = ColorGrading::cinematic();
        let vibrant = ColorGrading::vibrant();

        assert!(vibrant.saturation > cinematic.saturation);
        println!("✅ Color grading presets");
    }

    #[test]
    fn test_vignette_default() {
        let vignette = VignetteEffect::default();
        assert_eq!(vignette.intensity, 0.3);
        assert_eq!(vignette.roundness, 1.0);
        println!("✅ VignetteEffect default");
    }

    #[test]
    fn test_chromatic_aberration() {
        let ca = ChromaticAberrationEffect::default();
        assert_eq!(ca.intensity, 0.5);
        println!("✅ ChromaticAberrationEffect");
    }

    #[test]
    fn test_film_grain() {
        let fg = FilmGrainEffect::default();
        assert_eq!(fg.intensity, 0.1);
        println!("✅ FilmGrainEffect");
    }

    #[test]
    fn test_post_processing_builder() {
        let pp = PostProcessing::new()
            .with_bloom(BloomEffect::default())
            .with_dof(DepthOfFieldEffect::default())
            .with_tone_mapping(ToneMappingMode::ACES);

        assert!(pp.bloom.is_some());
        assert!(pp.depth_of_field.is_some());
        assert_eq!(pp.tone_mapping, ToneMappingMode::ACES);
        println!("✅ PostProcessing builder");
    }

    #[test]
    fn test_cinematic_preset() {
        let pp = PostProcessing::cinematic();
        assert!(pp.bloom.is_some());
        assert!(pp.depth_of_field.is_some());
        assert!(pp.color_grading.is_some());
        assert!(pp.vignette.is_some());
        assert_eq!(pp.tone_mapping, ToneMappingMode::ACES);
        println!("✅ Cinematic preset");
    }

    #[test]
    fn test_stylized_preset() {
        let pp = PostProcessing::stylized();
        assert!(pp.bloom.is_some());
        assert!(pp.color_grading.is_some());
        assert_eq!(pp.tone_mapping, ToneMappingMode::Reinhard);
        println!("✅ Stylized preset");
    }

    #[test]
    fn test_ssao_default() {
        let ssao = SSAOEffect::default();
        assert_eq!(ssao.radius, 0.5);
        assert_eq!(ssao.samples, 16);
        assert_eq!(ssao.intensity, 1.0);
        println!("✅ SSAO default");
    }

    #[test]
    fn test_ssao_high_quality() {
        let ssao = SSAOEffect::high_quality();
        assert_eq!(ssao.samples, 32);
        assert!(ssao.intensity > 1.0);
        println!("✅ SSAO high quality");
    }

    #[test]
    fn test_ssao_performance() {
        let ssao = SSAOEffect::performance();
        assert_eq!(ssao.samples, 8);
        println!("✅ SSAO performance");
    }

    #[test]
    fn test_hdr_default() {
        let hdr = HDRSettings::default();
        assert!(hdr.enabled);
        assert_eq!(hdr.exposure, 1.0);
        assert_eq!(hdr.white_point, 11.2);
        println!("✅ HDR default");
    }

    #[test]
    fn test_hdr_outdoor() {
        let hdr = HDRSettings::outdoor();
        assert!(hdr.enabled);
        assert!(hdr.exposure < 1.0);
        println!("✅ HDR outdoor");
    }

    #[test]
    fn test_hdr_indoor() {
        let hdr = HDRSettings::indoor();
        assert!(hdr.enabled);
        assert!(hdr.exposure > 1.0);
        println!("✅ HDR indoor");
    }

    #[test]
    fn test_post_processing_with_ssao() {
        let pp = PostProcessing::new().with_ssao(SSAOEffect::default());
        assert!(pp.ssao.is_some());
        println!("✅ PostProcessing with SSAO");
    }

    #[test]
    fn test_post_processing_with_hdr() {
        let pp = PostProcessing::new().with_hdr(HDRSettings::outdoor());
        assert!(pp.hdr.enabled);
        println!("✅ PostProcessing with HDR");
    }

    #[test]
    fn test_cinematic_has_ssao() {
        let pp = PostProcessing::cinematic();
        assert!(pp.ssao.is_some());
        assert!(pp.hdr.enabled);
        println!("✅ Cinematic has SSAO and HDR");
    }
}

