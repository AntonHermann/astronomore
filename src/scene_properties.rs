use wgpu::util::DeviceExt;

/// GPU-side representation of scene-level rendering parameters.
///
/// Memory layout matches the WGSL `SceneProperties` struct at `@group(3) @binding(0)`.
/// Vec3-equivalent fields use `[f32; 4]` so alignment requirements are satisfied
/// without separate padding fields in the struct.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ScenePropertiesUniform {
    /// Ambient (indirect) light contribution, in [0, 1].
    pub ambient_strength: f32,
    /// Lambertian diffuse contribution multiplier, in [0, 1].
    pub diffuse_factor: f32,
    /// Blinn-Phong specular contribution multiplier, in [0, 1].
    pub specular_intensity: f32,
    /// Blinn-Phong shininess exponent.
    pub shininess: f32,
    /// Non-zero: sample the diffuse texture. Zero: use `object_color` instead.
    pub use_texture: u32,
    pub _pad: [u32; 3],
    /// RGB light colour; w component is ignored by the shader.
    pub light_color: [f32; 4],
    /// Light position in world space; w component is ignored by the shader.
    pub light_position: [f32; 4],
    /// Flat object colour used when `use_texture == 0`; w component ignored.
    pub object_color: [f32; 4],
}

impl Default for ScenePropertiesUniform {
    fn default() -> Self {
        Self {
            ambient_strength: 0.05,
            diffuse_factor: 0.5,
            specular_intensity: 0.5,
            shininess: 64.0,
            use_texture: 1,
            _pad: [0; 3],
            light_color: [1.0, 1.0, 1.0, 0.0],
            light_position: [0.0, 0.0, 0.0, 0.0],
            object_color: [1.0, 1.0, 1.0, 0.0],
        }
    }
}

/// Scene-level rendering properties: CPU uniform, GPU buffer, and bind group.
pub struct SceneProperties {
    /// CPU-side data; mutated by the egui controls each frame.
    pub uniform: ScenePropertiesUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl SceneProperties {
    /// Create the bind group layout, initial GPU buffer, and bind group.
    pub fn new(device: &wgpu::Device) -> Self {
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene_properties_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform = ScenePropertiesUniform::default();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Scene Properties Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Scene Properties Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    /// Upload the current `uniform` to the GPU buffer.
    pub fn update(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
