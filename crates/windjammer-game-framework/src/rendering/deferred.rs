//! Deferred Rendering Pipeline
//!
//! Implements a deferred rendering system with G-Buffer for efficient lighting of many lights.
//! 
//! Deferred rendering separates geometry rendering from lighting calculations:
//! 1. Geometry Pass: Render scene to G-Buffer (position, normal, albedo, etc.)
//! 2. Lighting Pass: Calculate lighting using G-Buffer data
//!
//! Benefits:
//! - Support for many lights (100+) with minimal performance impact
//! - Decouples geometry complexity from lighting complexity
//! - Enables advanced lighting techniques (SSAO, SSR, etc.)

use crate::math::{Vec3, Vec4};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// G-Buffer texture formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GBufferFormat {
    /// Position (RGB: world position, A: depth)
    Position,
    /// Normal (RGB: world normal, A: unused)
    Normal,
    /// Albedo (RGB: base color, A: alpha)
    Albedo,
    /// Material properties (R: metallic, G: roughness, B: ao, A: unused)
    Material,
    /// Emissive (RGB: emissive color, A: intensity)
    Emissive,
}

impl GBufferFormat {
    /// Get the WGPU texture format for this G-Buffer component
    pub fn wgpu_format(&self) -> wgpu::TextureFormat {
        match self {
            GBufferFormat::Position => wgpu::TextureFormat::Rgba32Float,
            GBufferFormat::Normal => wgpu::TextureFormat::Rgba16Float,
            GBufferFormat::Albedo => wgpu::TextureFormat::Rgba8UnormSrgb,
            GBufferFormat::Material => wgpu::TextureFormat::Rgba8Unorm,
            GBufferFormat::Emissive => wgpu::TextureFormat::Rgba16Float,
        }
    }

    /// Get a descriptive name for this G-Buffer component
    pub fn name(&self) -> &'static str {
        match self {
            GBufferFormat::Position => "position",
            GBufferFormat::Normal => "normal",
            GBufferFormat::Albedo => "albedo",
            GBufferFormat::Material => "material",
            GBufferFormat::Emissive => "emissive",
        }
    }
}

/// G-Buffer configuration
#[derive(Debug, Clone)]
pub struct GBufferConfig {
    /// Width of G-Buffer textures
    pub width: u32,
    /// Height of G-Buffer textures
    pub height: u32,
    /// Enable position buffer
    pub enable_position: bool,
    /// Enable normal buffer
    pub enable_normal: bool,
    /// Enable albedo buffer
    pub enable_albedo: bool,
    /// Enable material buffer
    pub enable_material: bool,
    /// Enable emissive buffer
    pub enable_emissive: bool,
    /// Depth buffer format
    pub depth_format: wgpu::TextureFormat,
}

impl Default for GBufferConfig {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            enable_position: true,
            enable_normal: true,
            enable_albedo: true,
            enable_material: true,
            enable_emissive: true,
            depth_format: wgpu::TextureFormat::Depth32Float,
        }
    }
}

/// G-Buffer texture
pub struct GBufferTexture {
    /// The texture itself
    pub texture: wgpu::Texture,
    /// Texture view for rendering
    pub view: wgpu::TextureView,
    /// Texture format
    pub format: GBufferFormat,
}

/// G-Buffer for deferred rendering
pub struct GBuffer {
    /// Configuration
    pub config: GBufferConfig,
    /// Position texture
    pub position: Option<GBufferTexture>,
    /// Normal texture
    pub normal: Option<GBufferTexture>,
    /// Albedo texture
    pub albedo: Option<GBufferTexture>,
    /// Material properties texture
    pub material: Option<GBufferTexture>,
    /// Emissive texture
    pub emissive: Option<GBufferTexture>,
    /// Depth texture
    pub depth: wgpu::Texture,
    /// Depth texture view
    pub depth_view: wgpu::TextureView,
    /// Bind group for reading G-Buffer in lighting pass
    pub bind_group: wgpu::BindGroup,
    /// Bind group layout
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl GBuffer {
    /// Create a new G-Buffer
    pub fn new(device: &wgpu::Device, config: GBufferConfig) -> Self {
        // Create textures based on configuration
        let position = if config.enable_position {
            Some(Self::create_texture(
                device,
                &config,
                GBufferFormat::Position,
            ))
        } else {
            None
        };

        let normal = if config.enable_normal {
            Some(Self::create_texture(device, &config, GBufferFormat::Normal))
        } else {
            None
        };

        let albedo = if config.enable_albedo {
            Some(Self::create_texture(device, &config, GBufferFormat::Albedo))
        } else {
            None
        };

        let material = if config.enable_material {
            Some(Self::create_texture(
                device,
                &config,
                GBufferFormat::Material,
            ))
        } else {
            None
        };

        let emissive = if config.enable_emissive {
            Some(Self::create_texture(
                device,
                &config,
                GBufferFormat::Emissive,
            ))
        } else {
            None
        };

        // Create depth texture
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("G-Buffer Depth"),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

        // Create bind group layout
        let bind_group_layout = Self::create_bind_group_layout(device, &config);

        // Create bind group
        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &position,
            &normal,
            &albedo,
            &material,
            &emissive,
            &depth_view,
        );

        Self {
            config,
            position,
            normal,
            albedo,
            material,
            emissive,
            depth,
            depth_view,
            bind_group,
            bind_group_layout,
        }
    }

    /// Create a G-Buffer texture
    fn create_texture(
        device: &wgpu::Device,
        config: &GBufferConfig,
        format: GBufferFormat,
    ) -> GBufferTexture {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(&format!("G-Buffer {}", format.name())),
            size: wgpu::Extent3d {
                width: config.width,
                height: config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.wgpu_format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        GBufferTexture {
            texture,
            view,
            format,
        }
    }

    /// Create bind group layout for G-Buffer
    fn create_bind_group_layout(
        device: &wgpu::Device,
        config: &GBufferConfig,
    ) -> wgpu::BindGroupLayout {
        let mut entries = Vec::new();
        let mut binding = 0;

        // Add texture bindings based on configuration
        if config.enable_position {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            binding += 1;
        }

        if config.enable_normal {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            binding += 1;
        }

        if config.enable_albedo {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            binding += 1;
        }

        if config.enable_material {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            binding += 1;
        }

        if config.enable_emissive {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            });
            binding += 1;
        }

        // Add depth texture
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Depth,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });

        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("G-Buffer Bind Group Layout"),
            entries: &entries,
        })
    }

    /// Create bind group for G-Buffer
    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        position: &Option<GBufferTexture>,
        normal: &Option<GBufferTexture>,
        albedo: &Option<GBufferTexture>,
        material: &Option<GBufferTexture>,
        emissive: &Option<GBufferTexture>,
        depth_view: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        let mut entries = Vec::new();
        let mut binding = 0;

        if let Some(pos) = position {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(&pos.view),
            });
            binding += 1;
        }

        if let Some(norm) = normal {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(&norm.view),
            });
            binding += 1;
        }

        if let Some(alb) = albedo {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(&alb.view),
            });
            binding += 1;
        }

        if let Some(mat) = material {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(&mat.view),
            });
            binding += 1;
        }

        if let Some(emis) = emissive {
            entries.push(wgpu::BindGroupEntry {
                binding,
                resource: wgpu::BindingResource::TextureView(&emis.view),
            });
            binding += 1;
        }

        entries.push(wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::TextureView(depth_view),
        });

        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("G-Buffer Bind Group"),
            layout,
            entries: &entries,
        })
    }

    /// Resize the G-Buffer
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.config.width = width;
        self.config.height = height;

        // Recreate all textures
        if self.config.enable_position {
            self.position = Some(Self::create_texture(
                device,
                &self.config,
                GBufferFormat::Position,
            ));
        }

        if self.config.enable_normal {
            self.normal = Some(Self::create_texture(
                device,
                &self.config,
                GBufferFormat::Normal,
            ));
        }

        if self.config.enable_albedo {
            self.albedo = Some(Self::create_texture(
                device,
                &self.config,
                GBufferFormat::Albedo,
            ));
        }

        if self.config.enable_material {
            self.material = Some(Self::create_texture(
                device,
                &self.config,
                GBufferFormat::Material,
            ));
        }

        if self.config.enable_emissive {
            self.emissive = Some(Self::create_texture(
                device,
                &self.config,
                GBufferFormat::Emissive,
            ));
        }

        // Recreate depth texture
        self.depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("G-Buffer Depth"),
            size: wgpu::Extent3d {
                width: self.config.width,
                height: self.config.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.config.depth_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        self.depth_view = self.depth.create_view(&wgpu::TextureViewDescriptor::default());

        // Recreate bind group
        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            &self.position,
            &self.normal,
            &self.albedo,
            &self.material,
            &self.emissive,
            &self.depth_view,
        );
    }

    /// Get render pass color attachments for geometry pass
    pub fn get_color_attachments(&self) -> Vec<Option<wgpu::RenderPassColorAttachment>> {
        let mut attachments = Vec::new();

        if let Some(pos) = &self.position {
            attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: &pos.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        if let Some(norm) = &self.normal {
            attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: &norm.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        if let Some(alb) = &self.albedo {
            attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: &alb.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        if let Some(mat) = &self.material {
            attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: &mat.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        if let Some(emis) = &self.emissive {
            attachments.push(Some(wgpu::RenderPassColorAttachment {
                view: &emis.view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            }));
        }

        attachments
    }

    /// Get depth stencil attachment for geometry pass
    pub fn get_depth_attachment(&self) -> wgpu::RenderPassDepthStencilAttachment {
        wgpu::RenderPassDepthStencilAttachment {
            view: &self.depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
}

/// Light data for deferred lighting
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct DeferredLight {
    /// Light position (world space)
    pub position: [f32; 3],
    /// Light type (0 = point, 1 = directional, 2 = spot)
    pub light_type: u32,
    /// Light color
    pub color: [f32; 3],
    /// Light intensity
    pub intensity: f32,
    /// Light range (for point/spot lights)
    pub range: f32,
    /// Spot light inner cone angle (radians)
    pub inner_angle: f32,
    /// Spot light outer cone angle (radians)
    pub outer_angle: f32,
    /// Padding for alignment
    pub _padding: f32,
}

impl Default for DeferredLight {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            light_type: 0,
            color: [1.0, 1.0, 1.0],
            intensity: 1.0,
            range: 10.0,
            inner_angle: 0.0,
            outer_angle: 0.0,
            _padding: 0.0,
        }
    }
}

/// Deferred renderer
pub struct DeferredRenderer {
    /// G-Buffer
    pub gbuffer: GBuffer,
    /// Light buffer
    pub light_buffer: wgpu::Buffer,
    /// Maximum number of lights
    pub max_lights: usize,
    /// Current lights
    pub lights: Vec<DeferredLight>,
}

impl DeferredRenderer {
    /// Create a new deferred renderer
    pub fn new(device: &wgpu::Device, config: GBufferConfig, max_lights: usize) -> Self {
        let gbuffer = GBuffer::new(device, config);

        // Create light buffer
        let light_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Deferred Light Buffer"),
            size: (std::mem::size_of::<DeferredLight>() * max_lights) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            gbuffer,
            light_buffer,
            max_lights,
            lights: Vec::new(),
        }
    }

    /// Add a light
    pub fn add_light(&mut self, light: DeferredLight) {
        if self.lights.len() < self.max_lights {
            self.lights.push(light);
        }
    }

    /// Clear all lights
    pub fn clear_lights(&mut self) {
        self.lights.clear();
    }

    /// Update light buffer
    pub fn update_lights(&self, queue: &wgpu::Queue) {
        if !self.lights.is_empty() {
            queue.write_buffer(&self.light_buffer, 0, bytemuck::cast_slice(&self.lights));
        }
    }

    /// Resize the renderer
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        self.gbuffer.resize(device, width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gbuffer_format() {
        assert_eq!(
            GBufferFormat::Position.wgpu_format(),
            wgpu::TextureFormat::Rgba32Float
        );
        assert_eq!(
            GBufferFormat::Normal.wgpu_format(),
            wgpu::TextureFormat::Rgba16Float
        );
        assert_eq!(
            GBufferFormat::Albedo.wgpu_format(),
            wgpu::TextureFormat::Rgba8UnormSrgb
        );
    }

    #[test]
    fn test_gbuffer_config_default() {
        let config = GBufferConfig::default();
        assert_eq!(config.width, 1920);
        assert_eq!(config.height, 1080);
        assert!(config.enable_position);
        assert!(config.enable_normal);
        assert!(config.enable_albedo);
    }

    #[test]
    fn test_deferred_light_default() {
        let light = DeferredLight::default();
        assert_eq!(light.position, [0.0, 0.0, 0.0]);
        assert_eq!(light.color, [1.0, 1.0, 1.0]);
        assert_eq!(light.intensity, 1.0);
        assert_eq!(light.light_type, 0);
    }

    #[test]
    fn test_deferred_light_size() {
        // Ensure proper alignment for GPU buffer
        assert_eq!(std::mem::size_of::<DeferredLight>(), 48);
    }
}

