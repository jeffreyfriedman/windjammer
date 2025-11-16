// Texture Loader
// Loads textures from various formats (PNG, JPG, etc.) and uploads to GPU

use image::{DynamicImage, GenericImageView};
use std::collections::HashMap;
use std::path::Path;

/// Texture handle for referencing loaded textures
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(pub u32);

impl TextureHandle {
    /// Create a new texture handle
    pub fn new(id: u32) -> Self {
        Self(id)
    }
    
    /// Get the internal ID
    pub fn id(&self) -> u32 {
        self.0
    }
}

/// Texture format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// 8-bit RGBA
    Rgba8,
    /// 8-bit RGB
    Rgb8,
    /// 8-bit grayscale
    R8,
    /// 16-bit RGBA
    Rgba16,
    /// 32-bit float RGBA
    Rgba32Float,
    /// BC7 compressed (high quality)
    Bc7,
    /// BC1 compressed (RGB, low quality)
    Bc1,
    /// BC3 compressed (RGBA, medium quality)
    Bc3,
}

impl TextureFormat {
    /// Convert to wgpu texture format
    pub fn to_wgpu(&self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::Rgba8 => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureFormat::Rgb8 => wgpu::TextureFormat::Rgba8UnormSrgb, // wgpu doesn't support RGB8
            TextureFormat::R8 => wgpu::TextureFormat::R8Unorm,
            TextureFormat::Rgba16 => wgpu::TextureFormat::Rgba16Unorm,
            TextureFormat::Rgba32Float => wgpu::TextureFormat::Rgba32Float,
            TextureFormat::Bc7 => wgpu::TextureFormat::Bc7RgbaUnormSrgb,
            TextureFormat::Bc1 => wgpu::TextureFormat::Bc1RgbaUnormSrgb,
            TextureFormat::Bc3 => wgpu::TextureFormat::Bc3RgbaUnormSrgb,
        }
    }
    
    /// Get bytes per pixel (for uncompressed formats)
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            TextureFormat::Rgba8 => 4,
            TextureFormat::Rgb8 => 3,
            TextureFormat::R8 => 1,
            TextureFormat::Rgba16 => 8,
            TextureFormat::Rgba32Float => 16,
            _ => 0, // Compressed formats don't have a simple bytes per pixel
        }
    }
}

/// Texture filtering mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFilter {
    /// Nearest neighbor (pixelated)
    Nearest,
    /// Linear interpolation (smooth)
    Linear,
}

impl TextureFilter {
    /// Convert to wgpu filter mode
    pub fn to_wgpu(&self) -> wgpu::FilterMode {
        match self {
            TextureFilter::Nearest => wgpu::FilterMode::Nearest,
            TextureFilter::Linear => wgpu::FilterMode::Linear,
        }
    }
}

/// Texture wrap mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureWrap {
    /// Clamp to edge
    ClampToEdge,
    /// Repeat
    Repeat,
    /// Mirrored repeat
    MirrorRepeat,
}

impl TextureWrap {
    /// Convert to wgpu address mode
    pub fn to_wgpu(&self) -> wgpu::AddressMode {
        match self {
            TextureWrap::ClampToEdge => wgpu::AddressMode::ClampToEdge,
            TextureWrap::Repeat => wgpu::AddressMode::Repeat,
            TextureWrap::MirrorRepeat => wgpu::AddressMode::MirrorRepeat,
        }
    }
}

/// Texture configuration
#[derive(Debug, Clone)]
pub struct TextureConfig {
    /// Texture format
    pub format: TextureFormat,
    /// Minification filter
    pub min_filter: TextureFilter,
    /// Magnification filter
    pub mag_filter: TextureFilter,
    /// Mipmap filter
    pub mipmap_filter: TextureFilter,
    /// Wrap mode U
    pub wrap_u: TextureWrap,
    /// Wrap mode V
    pub wrap_v: TextureWrap,
    /// Generate mipmaps
    pub generate_mipmaps: bool,
    /// Enable anisotropic filtering
    pub anisotropic: bool,
    /// Max anisotropy level (1-16)
    pub max_anisotropy: u16,
}

impl Default for TextureConfig {
    fn default() -> Self {
        Self {
            format: TextureFormat::Rgba8,
            min_filter: TextureFilter::Linear,
            mag_filter: TextureFilter::Linear,
            mipmap_filter: TextureFilter::Linear,
            wrap_u: TextureWrap::Repeat,
            wrap_v: TextureWrap::Repeat,
            generate_mipmaps: true,
            anisotropic: true,
            max_anisotropy: 16,
        }
    }
}

impl TextureConfig {
    /// Create a new texture config with default settings
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set texture format
    pub fn with_format(mut self, format: TextureFormat) -> Self {
        self.format = format;
        self
    }
    
    /// Set filtering mode (min, mag, mipmap)
    pub fn with_filter(mut self, filter: TextureFilter) -> Self {
        self.min_filter = filter;
        self.mag_filter = filter;
        self.mipmap_filter = filter;
        self
    }
    
    /// Set wrap mode (U and V)
    pub fn with_wrap(mut self, wrap: TextureWrap) -> Self {
        self.wrap_u = wrap;
        self.wrap_v = wrap;
        self
    }
    
    /// Enable/disable mipmap generation
    pub fn with_mipmaps(mut self, generate: bool) -> Self {
        self.generate_mipmaps = generate;
        self
    }
    
    /// Enable/disable anisotropic filtering
    pub fn with_anisotropic(mut self, enabled: bool, max_level: u16) -> Self {
        self.anisotropic = enabled;
        self.max_anisotropy = max_level.clamp(1, 16);
        self
    }
}

/// Loaded texture data
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
    pub format: TextureFormat,
}

/// Texture loader for managing texture loading and caching
pub struct TextureLoader {
    device: wgpu::Device,
    queue: wgpu::Queue,
    textures: HashMap<TextureHandle, Texture>,
    path_to_handle: HashMap<String, TextureHandle>,
    next_id: u32,
}

impl TextureLoader {
    /// Create a new texture loader
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        Self {
            device,
            queue,
            textures: HashMap::new(),
            path_to_handle: HashMap::new(),
            next_id: 1, // Start at 1, reserve 0 for "no texture"
        }
    }
    
    /// Load a texture from a file path
    pub fn load_from_path(
        &mut self,
        path: impl AsRef<Path>,
        config: &TextureConfig,
    ) -> Result<TextureHandle, String> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        
        // Check if already loaded
        if let Some(&handle) = self.path_to_handle.get(&path_str) {
            return Ok(handle);
        }
        
        // Load image from file
        let img = image::open(path)
            .map_err(|e| format!("Failed to load image from {}: {}", path_str, e))?;
        
        // Load texture from image
        let handle = self.load_from_image(&img, config)?;
        
        // Cache path -> handle mapping
        self.path_to_handle.insert(path_str, handle);
        
        Ok(handle)
    }
    
    /// Load a texture from image data
    pub fn load_from_image(
        &mut self,
        img: &DynamicImage,
        config: &TextureConfig,
    ) -> Result<TextureHandle, String> {
        let (width, height) = img.dimensions();
        
        // Convert image to RGBA8 (most common format)
        let rgba = img.to_rgba8();
        let data = rgba.as_raw();
        
        // Create texture
        let texture = self.create_texture(
            data,
            width,
            height,
            config,
        )?;
        
        // Generate handle
        let handle = TextureHandle::new(self.next_id);
        self.next_id += 1;
        
        // Store texture
        self.textures.insert(handle, texture);
        
        Ok(handle)
    }
    
    /// Load a texture from raw bytes
    pub fn load_from_bytes(
        &mut self,
        bytes: &[u8],
        width: u32,
        height: u32,
        config: &TextureConfig,
    ) -> Result<TextureHandle, String> {
        // Create texture
        let texture = self.create_texture(
            bytes,
            width,
            height,
            config,
        )?;
        
        // Generate handle
        let handle = TextureHandle::new(self.next_id);
        self.next_id += 1;
        
        // Store texture
        self.textures.insert(handle, texture);
        
        Ok(handle)
    }
    
    /// Create a texture from raw data
    fn create_texture(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
        config: &TextureConfig,
    ) -> Result<Texture, String> {
        // Calculate mip level count
        let mip_level_count = if config.generate_mipmaps {
            (width.max(height) as f32).log2().floor() as u32 + 1
        } else {
            1
        };
        
        // Create texture
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Loaded Texture"),
            size,
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format.to_wgpu(),
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        
        // Upload data to texture (mip level 0)
        self.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width), // RGBA8 = 4 bytes per pixel
                rows_per_image: Some(height),
            },
            size,
        );
        
        // Generate mipmaps if requested
        if config.generate_mipmaps && mip_level_count > 1 {
            self.generate_mipmaps(&texture, mip_level_count, width, height);
        }
        
        // Create view
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        
        // Create sampler
        let sampler = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Texture Sampler"),
            address_mode_u: config.wrap_u.to_wgpu(),
            address_mode_v: config.wrap_v.to_wgpu(),
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: config.mag_filter.to_wgpu(),
            min_filter: config.min_filter.to_wgpu(),
            mipmap_filter: config.mipmap_filter.to_wgpu(),
            anisotropy_clamp: if config.anisotropic {
                config.max_anisotropy
            } else {
                1
            },
            ..Default::default()
        });
        
        Ok(Texture {
            texture,
            view,
            sampler,
            width,
            height,
            format: config.format,
        })
    }
    
    /// Generate mipmaps for a texture (simple box filter)
    fn generate_mipmaps(
        &self,
        _texture: &wgpu::Texture,
        _mip_level_count: u32,
        _width: u32,
        _height: u32,
    ) {
        // TODO: Implement mipmap generation using compute shader or blit
        // For now, we'll rely on the GPU driver to generate mipmaps
        // This is a placeholder for future implementation
    }
    
    /// Get a texture by handle
    pub fn get(&self, handle: TextureHandle) -> Option<&Texture> {
        self.textures.get(&handle)
    }
    
    /// Get a texture view by handle
    pub fn get_view(&self, handle: TextureHandle) -> Option<&wgpu::TextureView> {
        self.textures.get(&handle).map(|t| &t.view)
    }
    
    /// Get a texture sampler by handle
    pub fn get_sampler(&self, handle: TextureHandle) -> Option<&wgpu::Sampler> {
        self.textures.get(&handle).map(|t| &t.sampler)
    }
    
    /// Unload a texture
    pub fn unload(&mut self, handle: TextureHandle) {
        self.textures.remove(&handle);
        
        // Remove from path cache
        self.path_to_handle.retain(|_, &mut h| h != handle);
    }
    
    /// Clear all textures
    pub fn clear(&mut self) {
        self.textures.clear();
        self.path_to_handle.clear();
    }
    
    /// Get the number of loaded textures
    pub fn texture_count(&self) -> usize {
        self.textures.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_handle() {
        let handle1 = TextureHandle::new(1);
        let handle2 = TextureHandle::new(2);
        
        assert_eq!(handle1.id(), 1);
        assert_eq!(handle2.id(), 2);
        assert_ne!(handle1, handle2);
    }
    
    #[test]
    fn test_texture_config_builder() {
        let config = TextureConfig::new()
            .with_format(TextureFormat::Rgba8)
            .with_filter(TextureFilter::Nearest)
            .with_wrap(TextureWrap::ClampToEdge)
            .with_mipmaps(false)
            .with_anisotropic(false, 1);
        
        assert_eq!(config.format, TextureFormat::Rgba8);
        assert_eq!(config.min_filter, TextureFilter::Nearest);
        assert_eq!(config.wrap_u, TextureWrap::ClampToEdge);
        assert!(!config.generate_mipmaps);
        assert!(!config.anisotropic);
    }
    
    #[test]
    fn test_texture_format_conversion() {
        assert_eq!(
            TextureFormat::Rgba8.to_wgpu(),
            wgpu::TextureFormat::Rgba8UnormSrgb
        );
        assert_eq!(
            TextureFormat::R8.to_wgpu(),
            wgpu::TextureFormat::R8Unorm
        );
    }
}

