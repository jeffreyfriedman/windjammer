//! WGSL struct layout and automatic padding insertion
//!
//! WGSL has strict alignment rules that differ from Rust:
//! - Scalars (u32, i32, f32, bool): 4-byte alignment
//! - vec2: 8-byte alignment  
//! - vec3: 16-byte alignment (!)
//! - vec4: 16-byte alignment
//! - mat4x4: 16-byte alignment
//! - Structs: Aligned to largest member, minimum 16 bytes
//!
//! The compiler automatically inserts padding to match GPU requirements.

use crate::parser_impl::StructDecl;
use crate::codegen::wgsl::types::{WgslType, map_type_to_wgsl};
use anyhow::Result;

#[derive(Debug, Clone)]
pub struct LayoutField {
    pub name: String,
    pub wgsl_type: WgslType,
    pub offset: usize,
    pub is_padding: bool,
}

#[derive(Debug, Clone)]
pub struct StructLayout {
    pub name: String,
    pub fields: Vec<LayoutField>,
    pub total_size: usize,
    pub alignment: usize,
}

impl StructLayout {
    /// Calculate layout with automatic padding insertion
    pub fn from_struct_decl(decl: &StructDecl) -> Result<Self> {
        let mut fields = Vec::new();
        let mut current_offset = 0;
        let mut max_alignment = 1;
        let mut padding_counter = 0;

        for field in &decl.fields {
            let wgsl_type = map_type_to_wgsl(&field.field_type)?;
            let field_alignment = wgsl_type.alignment_bytes();
            let field_size = wgsl_type.size_bytes();

            max_alignment = max_alignment.max(field_alignment);

            // Check if we need padding to align this field
            let misalignment = current_offset % field_alignment;
            if misalignment != 0 {
                let padding_needed = field_alignment - misalignment;
                
                // Insert padding field(s)
                // WGSL doesn't have explicit padding types, so use f32 for 4-byte chunks
                let num_f32_pads = padding_needed / 4;
                for _ in 0..num_f32_pads {
                    fields.push(LayoutField {
                        name: format!("_pad{}", padding_counter),
                        wgsl_type: WgslType::F32,
                        offset: current_offset,
                        is_padding: true,
                    });
                    padding_counter += 1;
                    current_offset += 4;
                }
            }

            // Add the actual field
            fields.push(LayoutField {
                name: field.name.clone(),
                wgsl_type,
                offset: current_offset,
                is_padding: false,
            });

            current_offset += field_size;
        }

        // Structs must be aligned to their largest member alignment
        // Round up to next multiple of alignment
        let total_size = align_up(current_offset, max_alignment);
        
        // Add padding at end if needed
        if current_offset < total_size {
            let end_padding = total_size - current_offset;
            let num_f32_pads = end_padding / 4;
            for _ in 0..num_f32_pads {
                fields.push(LayoutField {
                    name: format!("_pad{}", padding_counter),
                    wgsl_type: WgslType::F32,
                    offset: current_offset,
                    is_padding: true,
                });
                padding_counter += 1;
                current_offset += 4;
            }
        }

        Ok(StructLayout {
            name: decl.name.clone(),
            fields,
            total_size,
            alignment: max_alignment,
        })
    }

    /// Generate WGSL struct declaration with padding
    pub fn to_wgsl_string(&self) -> String {
        let mut output = String::new();
        
        output.push_str("struct ");
        output.push_str(&self.name);
        output.push_str(" {\n");
        
        for field in &self.fields {
            output.push_str("    ");
            output.push_str(&field.name);
            output.push_str(": ");
            output.push_str(&field.wgsl_type.to_wgsl_string());
            output.push_str(",\n");
        }
        
        output.push_str("}");
        
        output
    }
}

/// Align value up to next multiple of alignment
fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_align_up() {
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(12, 16), 16);
        assert_eq!(align_up(13, 16), 16);
        assert_eq!(align_up(16, 16), 16);
        assert_eq!(align_up(17, 16), 32);
    }

    #[test]
    fn test_wgsl_type_alignment() {
        assert_eq!(WgslType::U32.alignment_bytes(), 4);
        assert_eq!(WgslType::Vec2F32.alignment_bytes(), 8);
        assert_eq!(WgslType::Vec3F32.alignment_bytes(), 16); // Critical!
        assert_eq!(WgslType::Vec4F32.alignment_bytes(), 16);
    }

    #[test]
    fn test_wgsl_type_size() {
        assert_eq!(WgslType::U32.size_bytes(), 4);
        assert_eq!(WgslType::Vec2F32.size_bytes(), 8);
        assert_eq!(WgslType::Vec3F32.size_bytes(), 12); // Size != alignment!
        assert_eq!(WgslType::Vec4F32.size_bytes(), 16);
    }
}
