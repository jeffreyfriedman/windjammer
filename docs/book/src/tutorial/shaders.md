# Shaders

<!-- feature: shaders -->

Real-time graphics and heavy parallel computation run on the graphics card, in small programs called shaders. Windjammer
compiles shader code to WGSL, the shading language of WebGPU, so the same toolchain that builds your application builds
its shaders, with type checking and without a separate build step.

There are two ways to write a shader, and a few tools around them.

| You write | You build it with | Use it when |
|---|---|---|
| a `.wjsl` file in WGSL-like syntax | `wj build FILE.wjsl --target wgsl` | you want shader code that reads like WGSL with Windjammer's checks |
| a `.wj` file with `@vertex`, `@fragment` or `@compute` functions | `wj build FILE.wj --target wgsl` | you want to write shaders in Windjammer syntax, with struct layout handled for you |
| a `shader Name { ... }` block | `wj shader-compile FILE.wjsl` | you want the compact form: declare the bindings and get a compute skeleton |

## WJSL: a compute shader

WJSL looks like WGSL with a few conveniences: `uniform` and `storage` bindings are declared in one line, and `let` and `mut`
replace `let` and `var`. This shader scales and offsets every number in a buffer, 64 at a time:

```sh,verify
$ cat > scale.wjsl <<'EOF'
struct Params {
    scale: f32,
    offset: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(0) @binding(0) uniform params: Params;
@group(0) @binding(1) storage read input: array<f32>;
@group(0) @binding(2) storage read_write output: array<f32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    output[i] = input[i] * params.scale + params.offset;
}
EOF
$ wj build scale.wjsl --target wgsl -o out
$ cat out/scale.wgsl
struct Params {
@group(0) @binding(0) var<uniform> params: Params;
@group(0) @binding(1) var<storage, read> input: array<f32>;
@group(0) @binding(2) var<storage, read_write> output: array<f32>;
@compute @workgroup_size(64, 1, 1)
output[i] = input[i] * params.scale + params.offset;
```

The bindings turned into the `var<uniform>` and `var<storage, ...>` declarations WGSL expects, with access modes from
`read` and `read_write`. The function body is carried over.

### A vertex and fragment pair

A graphics pipeline pairs a vertex shader, which places points, with a fragment shader, which colours pixels. Entry points
are marked with `@vertex` and `@fragment`, and their inputs and outputs with `@location` and `@builtin`:

```sh,verify
$ cat > triangle.wjsl <<'EOF'
struct VertexOut {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) color: vec3<f32>) -> VertexOut {
    var out: VertexOut;
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
EOF
$ wj build triangle.wjsl --target wgsl -o out
$ cat out/triangle.wgsl
@vertex
fn vs_main(@location(0) pos: vec2<f32>, @location(1) color: vec3<f32>) -> VertexOut {
@fragment
return vec4<f32>(in.color, 1.0);
```

### Checking shaders

`wj validate-wjsl DIR` transpiles every `.wjsl` file under a directory and reports the ones with errors, which makes it a
good step in a build or a test. Errors name the problem and what is in scope:

```sh,verify
$ mkdir -p shaders
$ cat > shaders/ok.wjsl <<'EOF'
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = 1.0 + 2.0;
}
EOF
$ wj validate-wjsl shaders
1 shader(s) validated successfully
$ cat > shaders/bad.wjsl <<'EOF'
@compute @workgroup_size(1, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let x = missing_name + 1.0;
}
EOF
$ wj validate-wjsl shaders 2>&1 || true
Unknown identifier 'missing_name'
1 error(s)
```

Shared code lives in files you pull in with `use "other.wjsl"`; the includes are resolved before checking.

### The compact block form

For a quick compute skeleton, declare the bindings in a `shader` block and `wj shader-compile` writes the struct, the
bindings and an entry point with the bounds check:

```sh,verify
$ cat > blur.wjsl <<'EOF'
shader Blur {
    uniform screen_size: Vec2<f32>
    storage output: array<Vec4<f32>>
}
EOF
$ wj shader-compile blur.wjsl
struct Uniforms {
screen_size: vec2<f32>,
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> output: array<vec4<f32>>;
@compute @workgroup_size(8, 8)
```

## Shaders in Windjammer syntax

The second route writes shaders as ordinary Windjammer: structs, functions, `let` and `if`, with `@vertex`, `@fragment` and
`@compute(workgroup_size = [8, 8, 1])` on the entry points. Windjammer's numeric types map to WGSL's: `uint` is `u32`,
`float` is `f32`, `vec2<float>` is `vec2<f32>`.

```sh,verify
$ cat > colour.wj <<'EOF'
pub struct VertexOutput {
    position: vec4<float>,
    color: vec4<float>,
}

@vertex
pub fn vs_main(@location(0) pos: vec2<float>) -> VertexOutput {
    VertexOutput {
        position: vec4(pos.x, pos.y, 0.0, 1.0),
        color: vec4(1.0, 1.0, 1.0, 1.0),
    }
}

@fragment
pub fn fs_main(@location(0) color: vec4<float>) -> @location(0) vec4<float> {
    color
}
EOF
$ wj build colour.wj --target wgsl -o out
$ cat out/colour.wgsl
struct VertexOutput {
@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> VertexOutput {
@fragment
fn fs_main(@location(0) color: vec4<f32>) -> vec4<f32> {
    return color;
```

The last expression of a function became the `return`, as it does everywhere in Windjammer.

### Struct layout the GPU agrees with

Data crosses from the CPU to the GPU in structs, and the GPU lays them out under strict alignment rules: a `vec3` takes
16-byte slots, for instance. Windjammer inserts the padding for you, so the struct you write and the struct the shader
sees agree:

```sh,verify
$ cat > layout.wj <<'EOF'
pub struct Layout1 {
    count: uint,
    position: vec2<float>,
    dir: vec3<float>,
    tint: vec4<float>,
}
EOF
$ wj build layout.wj --target wgsl -o out
$ cat out/layout.wgsl
count: u32,
_pad0: f32,
position: vec2<f32>,
dir: vec3<f32>,
_pad1: f32,
tint: vec4<f32>,
```

`_pad0` and `_pad1` keep `position` and `tint` on their required boundaries. The same layout is what host code uses to fill
buffers, so a mismatch cannot creep in between the two sides. A helper in the compiler also reads bindings and entry
points back out of generated WGSL, which host-side crates use to set up pipelines.

### Uniform-safe types

Uniform buffers do not take unsigned integers in every position, so a `uniform` declared with `uint` types is converted to
float, and the generated code converts back where the value is used. The conversion is marked in the output:

```sh,verify
$ cat > sizes.wj <<'EOF'
@uniform
@binding(3)
extern let screen_size: vec2<uint>;

@compute(workgroup_size = [8, 8, 1])
fn main(@builtin(global_invocation_id) id: vec3<uint>) {
    let w = screen_size.x as uint;
}
EOF
$ wj build sizes.wj --target wgsl -o out
$ cat out/sizes.wgsl
auto-converted from vec2<u32>
let w = u32(screen_size.x);
```

## What comes next

Shaders run on the GPU, so running one is the job of your host program and a graphics library. Windjammer's part is the
language and the checks: one toolchain, one set of diagnostics, and generated code you can read. Windjammer source for the
CPU side is covered by the rest of this book, and the build targets for it are in [Targets](targets.md).
