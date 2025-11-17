//! GPU-Accelerated Skeletal Animation System
//!
//! Provides GPU skinning for high-performance skeletal animation.

use crate::animation::{Animation, AnimationPlayer, Skeleton};
use crate::math::Mat4;
use std::sync::Arc;

/// Maximum number of bones supported per skeleton
pub const MAX_BONES: usize = 256;

/// GPU animation system
pub struct AnimationGPUSystem {
    /// WGPU device (for buffer creation)
    device: Arc<wgpu::Device>,
    
    /// WGPU queue (for buffer updates)
    queue: Arc<wgpu::Queue>,
    
    /// Bone matrix buffers (one per animated entity)
    bone_buffers: Vec<BoneMatrixBuffer>,
}

impl AnimationGPUSystem {
    /// Create a new GPU animation system
    pub fn new(device: Arc<wgpu::Device>, queue: Arc<wgpu::Queue>) -> Self {
        Self {
            device,
            queue,
            bone_buffers: Vec::new(),
        }
    }
    
    /// Create a bone matrix buffer for an animated entity
    pub fn create_bone_buffer(&mut self) -> usize {
        let buffer = BoneMatrixBuffer::new(&self.device);
        let index = self.bone_buffers.len();
        self.bone_buffers.push(buffer);
        index
    }
    
    /// Update bone matrices for an entity
    pub fn update_bones(
        &mut self,
        buffer_index: usize,
        skeleton: &Skeleton,
        player: &AnimationPlayer,
    ) {
        if buffer_index >= self.bone_buffers.len() {
            return;
        }
        
        // Calculate bone matrices from current animation state
        let bone_matrices = player.calculate_skinning_matrices(skeleton);
        
        // Update GPU buffer
        self.bone_buffers[buffer_index].update(&self.queue, &bone_matrices);
    }
    
    /// Get bone matrix buffer for rendering
    pub fn get_bone_buffer(&self, buffer_index: usize) -> Option<&wgpu::Buffer> {
        self.bone_buffers.get(buffer_index).map(|b| &b.buffer)
    }
    
    /// Get bind group for bone matrices
    pub fn get_bone_bind_group(&self, buffer_index: usize) -> Option<&wgpu::BindGroup> {
        self.bone_buffers.get(buffer_index).map(|b| &b.bind_group)
    }
}

/// Bone matrix buffer (GPU-side storage)
struct BoneMatrixBuffer {
    /// GPU buffer containing bone matrices
    buffer: wgpu::Buffer,
    
    /// Bind group for shader access
    bind_group: wgpu::BindGroup,
}

impl BoneMatrixBuffer {
    /// Create a new bone matrix buffer
    fn new(device: &wgpu::Device) -> Self {
        // Create buffer for bone matrices (256 mat4x4)
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Bone Matrix Buffer"),
            size: (MAX_BONES * std::mem::size_of::<Mat4>()) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        // Create bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bone Matrix Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        
        // Create bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bone Matrix Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        
        Self {
            buffer,
            bind_group,
        }
    }
    
    /// Update bone matrices in GPU buffer
    fn update(&self, queue: &wgpu::Queue, matrices: &[Mat4]) {
        // Ensure we don't exceed maximum bones
        let count = matrices.len().min(MAX_BONES);
        
        // Convert Mat4 to raw bytes (Mat4 is column-major, which is what shaders expect)
        let mut data = Vec::with_capacity(MAX_BONES * 16 * 4); // 16 floats per matrix, 4 bytes per float
        for i in 0..count {
            // Convert Mat4 to array of floats
            let matrix_array: &[f32; 16] = matrices[i].as_ref();
            let matrix_bytes = bytemuck::cast_slice(matrix_array);
            data.extend_from_slice(matrix_bytes);
        }
        
        // Pad with identity matrices if needed
        let identity = Mat4::IDENTITY;
        let identity_array: &[f32; 16] = identity.as_ref();
        let identity_bytes = bytemuck::cast_slice(identity_array);
        for _ in count..MAX_BONES {
            data.extend_from_slice(identity_bytes);
        }
        
        // Write to GPU buffer
        queue.write_buffer(&self.buffer, 0, &data);
    }
}

/// Skinned mesh vertex (for GPU skinning)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SkinnedVertex {
    /// Position
    pub position: [f32; 3],
    
    /// Normal
    pub normal: [f32; 3],
    
    /// UV coordinates
    pub uv: [f32; 2],
    
    /// Tangent (with handedness in w)
    pub tangent: [f32; 4],
    
    /// Bone indices (up to 4 bones per vertex)
    pub bone_indices: [u32; 4],
    
    /// Bone weights (should sum to 1.0)
    pub bone_weights: [f32; 4],
}

impl SkinnedVertex {
    /// Vertex buffer layout descriptor
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SkinnedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // UV
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2) as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
                // Tangent
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2 + std::mem::size_of::<[f32; 2]>())
                        as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // Bone indices
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2
                        + std::mem::size_of::<[f32; 2]>()
                        + std::mem::size_of::<[f32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Uint32x4,
                },
                // Bone weights
                wgpu::VertexAttribute {
                    offset: (std::mem::size_of::<[f32; 3]>() * 2
                        + std::mem::size_of::<[f32; 2]>()
                        + std::mem::size_of::<[f32; 4]>()
                        + std::mem::size_of::<[u32; 4]>()) as wgpu::BufferAddress,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Extension trait for AnimationPlayer to calculate skinning matrices
pub trait AnimationPlayerExt {
    /// Calculate skinning matrices for GPU
    fn calculate_skinning_matrices(&self, skeleton: &Skeleton) -> Vec<Mat4>;
}

impl AnimationPlayerExt for AnimationPlayer {
    fn calculate_skinning_matrices(&self, skeleton: &Skeleton) -> Vec<Mat4> {
        // Get current bone transforms from animation
        let bone_transforms = skeleton.calculate_bone_matrices();
        
        // Multiply by inverse bind pose to get skinning matrices
        let mut skinning_matrices = Vec::with_capacity(skeleton.bones.len());
        for (i, bone) in skeleton.bones.iter().enumerate() {
            let skinning_matrix = bone_transforms[i] * bone.inverse_bind_pose;
            skinning_matrices.push(skinning_matrix);
        }
        
        skinning_matrices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_skinned_vertex_size() {
        // Verify vertex size is correct
        let expected_size = std::mem::size_of::<[f32; 3]>() * 2 // position + normal
            + std::mem::size_of::<[f32; 2]>() // uv
            + std::mem::size_of::<[f32; 4]>() // tangent
            + std::mem::size_of::<[u32; 4]>() // bone_indices
            + std::mem::size_of::<[f32; 4]>(); // bone_weights
        
        assert_eq!(std::mem::size_of::<SkinnedVertex>(), expected_size);
    }
    
    #[test]
    fn test_max_bones() {
        // Ensure MAX_BONES is reasonable
        assert_eq!(MAX_BONES, 256);
        assert!(MAX_BONES > 0);
    }
}

