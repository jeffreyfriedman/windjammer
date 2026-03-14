




use crate::math::vec3::Vec3;
use crate::math::mat4::Mat4;
use crate::ecs::entity::Entity;
use crate::ecs::components::Transform;

#[derive(Debug, Clone)]
pub struct CameraData {
    pub view_matrix: Mat4,
    pub proj_matrix: Mat4,
    pub position: Vec3,
    pub screen_width: f32,
    pub screen_height: f32,
    pub near_plane: f32,
    pub far_plane: f32,
}

#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub direction: Vec3,
    pub color: Vec3,
    pub intensity: f32,
}

#[derive(Debug, Clone)]
pub struct AmbientLight {
    pub sky_color: Vec3,
    pub ground_color: Vec3,
    pub intensity: f32,
}

#[derive(Debug, Clone)]
pub struct LightingData {
    pub sun: DirectionalLight,
    pub ambient: AmbientLight,
    pub gi_samples: u32,
    pub gi_intensity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PostProcessingData {
    pub exposure: f32,
    pub gamma: f32,
    pub bloom_threshold: f32,
    pub vignette_strength: f32,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct VoxelWorldData {
    pub svo_nodes: Vec<u32>,
    pub world_size: f32,
    pub depth: u32,
}

#[derive(Debug, Clone)]
pub struct MaterialData {
    pub id: u32,
    pub albedo: Vec3,
    pub roughness: f32,
    pub metallic: f32,
    pub emission: Vec3,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct RenderMeshData {
    pub mesh_name: String,
    pub material_name: String,
}

impl RenderMeshData {
#[inline]
pub fn new(mesh_name: String, material_name: String) -> RenderMeshData {
        RenderMeshData { mesh_name, material_name }
}
}

pub trait RenderPort {
    fn initialize(&mut self);
    fn set_camera(&mut self, camera: CameraData);
    fn set_lighting(&mut self, lighting: LightingData);
    fn set_post_processing(&mut self, config: PostProcessingData);
    fn upload_voxel_world(&mut self, world: VoxelWorldData);
    fn upload_materials(&mut self, materials: Vec<MaterialData>);
    fn render_voxels(&mut self);
    fn render_mesh(&mut self, entity: Entity, mesh: RenderMeshData, transform: Transform);
    fn render_frame(&mut self);
    fn get_output_buffer(&mut self) -> Vec<u8>;
    fn shutdown(&mut self);
}

#[derive(Debug, Clone, Default)]
pub struct MockRenderer {
    pub frames_rendered: u32,
    pub camera_set: bool,
    pub lighting_set: bool,
    pub world_uploaded: bool,
    pub voxel_rendered: bool,
    pub draw_calls: Vec<Entity>,
}

impl MockRenderer {
#[inline]
pub fn new() -> MockRenderer {
        MockRenderer { frames_rendered: 0, camera_set: false, lighting_set: false, world_uploaded: false, voxel_rendered: false, draw_calls: Vec::new() }
}
}

impl RenderPort for MockRenderer {
#[inline]
fn initialize(&mut self) {
}
#[inline]
fn set_camera(&mut self, __camera: CameraData) {
        self.camera_set = true;
}
#[inline]
fn set_lighting(&mut self, __lighting: LightingData) {
        self.lighting_set = true;
}
#[inline]
fn set_post_processing(&mut self, __config: PostProcessingData) {
}
#[inline]
fn upload_voxel_world(&mut self, __world: VoxelWorldData) {
        self.world_uploaded = true;
}
#[inline]
fn upload_materials(&mut self, __materials: Vec<MaterialData>) {
}
#[inline]
fn render_voxels(&mut self) {
        self.voxel_rendered = true;
}
#[inline]
fn render_mesh(&mut self, entity: Entity, __mesh: RenderMeshData, __transform: Transform) {
        self.draw_calls.push(entity);
}
#[inline]
fn render_frame(&mut self) {
        self.frames_rendered += 1;
}
#[inline]
fn get_output_buffer(&mut self) -> Vec<u8> {
        Vec::new()
}
#[inline]
fn shutdown(&mut self) {
}
}

/// Create LightingData from flat lighting config (used by voxel games)
#[inline]
pub fn lighting_from_config(sun_dir_x: f32, sun_dir_y: f32, sun_dir_z: f32, sun_color_r: f32, sun_color_g: f32, sun_color_b: f32, sun_intensity: f32, sky_color_r: f32, sky_color_g: f32, sky_color_b: f32, ground_color_r: f32, ground_color_g: f32, ground_color_b: f32, ambient_intensity: f32, gi_samples: u32, gi_intensity: f32) -> LightingData {
    LightingData { sun: DirectionalLight { direction: Vec3::new(sun_dir_x, sun_dir_y, sun_dir_z), color: Vec3::new(sun_color_r, sun_color_g, sun_color_b), intensity: sun_intensity }, ambient: AmbientLight { sky_color: Vec3::new(sky_color_r, sky_color_g, sky_color_b), ground_color: Vec3::new(ground_color_r, ground_color_g, ground_color_b), intensity: ambient_intensity }, gi_samples, gi_intensity }
}

/// Create PostProcessingData from flat config
#[inline]
pub fn post_processing_from_config(exposure: f32, gamma: f32, bloom_threshold: f32, vignette_strength: f32) -> PostProcessingData {
    PostProcessingData { exposure, gamma, bloom_threshold, vignette_strength }
}

#[inline]
pub fn run_rendering_system<T>(entities: Vec<(Entity, RenderMeshData, Transform)>, mut renderer: T, camera: CameraData) -> T
where
    T: RenderPort {
    renderer.set_camera(camera);
    for (entity, mesh, transform) in entities {
        renderer.render_mesh(entity, mesh, transform);
    }
    renderer.render_frame();
    renderer
}

