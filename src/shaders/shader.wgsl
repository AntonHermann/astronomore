// ==== Vertex shader ====

struct CameraUniform {
    /// Combined view and projection matrix, mapping from world space to clip space.
    view_proj: mat4x4<f32>,
    /// Camera position in world space
    camera_pos: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct ModelUniform {
    /// Transform from model space to world space.
    model_to_world_transform: mat4x4<f32>,
}
@group(2) @binding(0)
var<uniform> model_transform: ModelUniform;

struct SceneProperties {
    ambient_strength: f32,
    diffuse_factor: f32,
    specular_intensity: f32,
    shininess: f32,
    use_texture: u32,
    pad0: u32,
    pad1: u32,
    pad2: u32,
    /// RGB light colour; w ignored.
    light_color: vec4<f32>,
    /// Light position in world space; w ignored.
    light_position: vec4<f32>,
    /// Flat object colour used when use_texture == 0; w ignored.
    object_color: vec4<f32>,
}
@group(3) @binding(0)
var<uniform> scene_props: SceneProperties;

struct VertexInput {
    /// Position of the vertex in model space.
    @location(0) position: vec3<f32>,
    /// Texture coordinates for the vertex, used to sample the texture in the shader.
    @location(1) tex_coords: vec2<f32>,
    /// Normal vector at the vertex, used for lighting calculations in the shader.
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    /// Vertex position in clip space
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) body_position: vec4<f32>,
    @location(2) body_normal: vec4<f32>,
};

@vertex
fn vs_main(
    model: VertexInput
) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    let body_pos: vec4<f32> = model_transform.model_to_world_transform * vec4<f32>(model.position, 1.0);
    out.body_position = body_pos;
    // NOTE: This only works if the model matrix has no non-uniform scaling!
    out.body_normal = model_transform.model_to_world_transform * vec4<f32>(model.normal, 0.0);
    out.clip_position = camera.view_proj * body_pos;
    return out;
}


// ==== Fragment shader ====

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Always call textureSample unconditionally so the texture() call lands in
    // uniform control flow. GLSL ES / WebGL2 compilers reject texture() inside
    // an if-block even when the condition is dynamically uniform.
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords).rgb;
    let base_color = select(scene_props.object_color.xyz, tex_color, scene_props.use_texture != 0u);

    let light_color = scene_props.light_color.xyz;
    let light_pos   = scene_props.light_position.xyz;

    // Re-normalize after interpolation across the triangle.
    let N = normalize(in.body_normal.xyz);
    let L = normalize(light_pos - in.body_position.xyz);
    let V = normalize(camera.camera_pos - in.body_position.xyz);
    let H = normalize(L + V);

    // ==== Ambient ====
    let ambient_color = light_color * scene_props.ambient_strength;

    // ==== Diffuse ====
    let diffuse_color = light_color * scene_props.diffuse_factor * max(dot(L, N), 0.0);

    // ==== Specular (Blinn-Phong) ====
    let specular_color = light_color * scene_props.specular_intensity
        * pow(max(dot(H, N), 0.0), scene_props.shininess);

    let out_color = (ambient_color + diffuse_color + specular_color) * base_color;

    return vec4<f32>(out_color, 1.0);
}

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
