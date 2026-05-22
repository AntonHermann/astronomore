// ==== Vertex shader ====

struct CameraUniform {
    /// Combined view and projection matrix, mapping from world space to clip space.
    view_proj: mat4x4<f32>,
}
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct ModelUniform {
    /// Transform from model space to world space.
    model_to_world_transform: mat4x4<f32>,
}
@group(2) @binding(0)
var<uniform> model_transform: ModelUniform;

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
    let object_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);

    let sun_color = vec3<f32>(1.0, 1.0, 1.0);
    let sun_pos = vec4<f32>(0., 0., 0., 1.);

    // ==== Ambient Color ==== //
    let ambient_strength = 0.1;
    let ambient_color = sun_color * ambient_strength;
    
    // ==== Diffuse Color ==== //
    let light_dir = sun_pos - in.body_position;
    let diffuse_strength = max(dot(light_dir, in.body_normal), 0);
    let diffuse_color = sun_color * diffuse_strength;
    
    var out_color = (ambient_color + diffuse_color) * object_color.rgb;

    return vec4<f32>(out_color, object_color.a);
}

@fragment
fn fs_wireframe(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.0, 0.0, 0.0, 1.0);
}
