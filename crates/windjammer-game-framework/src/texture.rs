//! Texture loading and management

use wgpu::util::DeviceExt;

/// Texture handle
pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
    pub width: u32,
    pub height: u32,
}

impl Texture {
    /// Create a texture from raw RGBA bytes
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        width: u32,
        height: u32,
        label: Option<&str>,
    ) -> Result<Self, String> {
        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
            width,
            height,
        })
    }

    /// Create a solid color texture
    pub fn from_color(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        color: [u8; 4],
        label: Option<&str>,
    ) -> Result<Self, String> {
        // Create a 1x1 texture with the given color
        Self::from_bytes(device, queue, &color, 1, 1, label)
    }

    /// Create a checkerboard pattern texture (useful for testing)
    pub fn checkerboard(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        size: u32,
        label: Option<&str>,
    ) -> Result<Self, String> {
        let mut bytes = Vec::with_capacity((size * size * 4) as usize);

        for y in 0..size {
            for x in 0..size {
                let is_white = (x / 8 + y / 8) % 2 == 0;
                let color = if is_white {
                    [255, 255, 255, 255]
                } else {
                    [128, 128, 128, 255]
                };
                bytes.extend_from_slice(&color);
            }
        }

        Self::from_bytes(device, queue, &bytes, size, size, label)
    }
}

/// Texture atlas for sprite sheets
pub struct TextureAtlas {
    pub texture: Texture,
    pub tile_width: u32,
    pub tile_height: u32,
    pub columns: u32,
    pub rows: u32,
}

impl TextureAtlas {
    /// Create a texture atlas from raw bytes
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        width: u32,
        height: u32,
        tile_width: u32,
        tile_height: u32,
        label: Option<&str>,
    ) -> Result<Self, String> {
        let texture = Texture::from_bytes(device, queue, bytes, width, height, label)?;

        let columns = width / tile_width;
        let rows = height / tile_height;

        Ok(Self {
            texture,
            tile_width,
            tile_height,
            columns,
            rows,
        })
    }

    /// Get UV coordinates for a specific tile
    pub fn get_tile_uv(&self, tile_index: u32) -> [f32; 4] {
        let col = tile_index % self.columns;
        let row = tile_index / self.columns;

        let u = (col * self.tile_width) as f32 / self.texture.width as f32;
        let v = (row * self.tile_height) as f32 / self.texture.height as f32;
        let u2 = ((col + 1) * self.tile_width) as f32 / self.texture.width as f32;
        let v2 = ((row + 1) * self.tile_height) as f32 / self.texture.height as f32;

        [u, v, u2, v2]
    }

    /// Get total number of tiles
    pub fn tile_count(&self) -> u32 {
        self.columns * self.rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_texture_atlas_uv() {
        // Mock texture atlas: 64x64 texture with 16x16 tiles = 4x4 grid
        let atlas = TextureAtlas {
            texture: unsafe { std::mem::zeroed() }, // We only test UV calculations
            tile_width: 16,
            tile_height: 16,
            columns: 4,
            rows: 4,
        };

        // Manually set texture dimensions for testing
        let atlas = TextureAtlas {
            texture: Texture {
                texture: unsafe { std::mem::zeroed() },
                view: unsafe { std::mem::zeroed() },
                sampler: unsafe { std::mem::zeroed() },
                width: 64,
                height: 64,
            },
            tile_width: 16,
            tile_height: 16,
            columns: 4,
            rows: 4,
        };

        // Test first tile (0, 0)
        let uv = atlas.get_tile_uv(0);
        assert_eq!(uv, [0.0, 0.0, 0.25, 0.25]);

        // Test second tile (1, 0)
        let uv = atlas.get_tile_uv(1);
        assert_eq!(uv, [0.25, 0.0, 0.5, 0.25]);

        // Test tile count
        assert_eq!(atlas.tile_count(), 16);
    }
}
