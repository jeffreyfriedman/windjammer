//! WGSL built-in function names (W3C WGSL spec, section "Built-in Functions").
//!
//! The table is the spec's list, grouped as the spec groups it, so a call can be
//! classified as builtin / user function / undefined. A name missing here would
//! become a false "undefined function" (WJ0195), so this list is complete for the
//! spec including atomics, derivatives, packing, texture and subgroup functions.

/// Every WGSL built-in function, grouped as the spec groups them.
pub const WGSL_BUILTINS: &[&str] = &[
    // Logical and array
    "all", "any", "arrayLength", "bitcast", "select",
    // Numeric
    "abs", "acos", "acosh", "asin", "asinh", "atan", "atan2", "atanh", "ceil", "clamp", "cos",
    "cosh", "countLeadingZeros", "countOneBits", "countTrailingZeros", "cross", "degrees",
    "determinant", "distance", "dot", "dot4I8Packed", "dot4U8Packed", "exp", "exp2",
    "extractBits", "faceForward", "firstLeadingBit", "firstTrailingBit", "floor", "fma", "fract",
    "frexp", "insertBits", "inverseSqrt", "ldexp", "length", "log", "log2", "max", "min", "mix",
    "modf", "normalize", "pow", "quantizeToF16", "radians", "reflect", "refract", "reverseBits",
    "round", "saturate", "sign", "sin", "sinh", "smoothstep", "sqrt", "step", "tan", "tanh",
    "transpose", "trunc",
    // Derivatives
    "dpdx", "dpdxCoarse", "dpdxFine", "dpdy", "dpdyCoarse", "dpdyFine", "fwidth", "fwidthCoarse",
    "fwidthFine",
    // Texture
    "textureDimensions", "textureGather", "textureGatherCompare", "textureLoad", "textureNumLayers",
    "textureNumLevels", "textureNumSamples", "textureSample", "textureSampleBaseClampToEdge",
    "textureSampleBias", "textureSampleCompare", "textureSampleCompareLevel", "textureSampleGrad",
    "textureSampleLevel", "textureStore",
    // Atomic
    "atomicAdd", "atomicAnd", "atomicCompareExchangeWeak", "atomicExchange", "atomicLoad",
    "atomicMax", "atomicMin", "atomicOr", "atomicStore", "atomicSub", "atomicXor",
    // Data packing and unpacking
    "pack2x16float", "pack2x16snorm", "pack2x16unorm", "pack4x8snorm", "pack4x8unorm",
    "pack4xI8", "pack4xI8Clamp", "pack4xU8", "pack4xU8Clamp", "unpack2x16float",
    "unpack2x16snorm", "unpack2x16unorm", "unpack4x8snorm", "unpack4x8unorm", "unpack4xI8",
    "unpack4xU8",
    // Synchronization
    "storageBarrier", "textureBarrier", "workgroupBarrier", "workgroupUniformLoad",
    // Subgroup and quad
    "quadBroadcast", "quadSwapDiagonal", "quadSwapX", "quadSwapY", "subgroupAdd", "subgroupAll",
    "subgroupAnd", "subgroupAny", "subgroupBallot", "subgroupBroadcast", "subgroupBroadcastFirst",
    "subgroupElect", "subgroupExclusiveAdd", "subgroupExclusiveMul", "subgroupInclusiveAdd",
    "subgroupInclusiveMul", "subgroupMax", "subgroupMin", "subgroupMul", "subgroupOr",
    "subgroupShuffle", "subgroupShuffleDown", "subgroupShuffleUp", "subgroupShuffleXor",
    "subgroupXor",
];

/// True when `name` is a WGSL built-in function.
pub fn is_builtin_function(name: &str) -> bool {
    WGSL_BUILTINS.contains(&name)
}

/// True for WGSL value-constructor spellings that reach the checker as plain
/// identifiers: `vec3f`, `vec2u`, `mat4x4f`, `array`, and the predeclared aliases.
pub fn is_constructor_name(name: &str) -> bool {
    if matches!(name, "array" | "bool" | "f16" | "f32" | "i32" | "u32") {
        return true;
    }
    let rest = if let Some(r) = name.strip_prefix("vec") {
        r
    } else if let Some(r) = name.strip_prefix("mat") {
        r
    } else {
        return false;
    };
    // vec2f, vec3u, vec4i, vec2h; matCxRf / matCxRh (the unsuffixed forms are tokens)
    let bytes = rest.as_bytes();
    let dims = if name.starts_with("vec") { 1 } else { 3 };
    bytes.len() == dims + 1
        && matches!(bytes[dims], b'f' | b'h' | b'i' | b'u')
        && bytes[..dims].iter().enumerate().all(|(i, b)| {
            if dims == 3 && i == 1 { *b == b'x' } else { (b'2'..=b'4').contains(b) }
        })
}
