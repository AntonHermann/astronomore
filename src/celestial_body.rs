use wgpu::util::DeviceExt;

use crate::{mesh, texture, transform};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub model_matrix: [[f32; 4]; 4],
}
impl ModelUniform {
    pub fn from_transform(transform: &transform::Transform) -> Self {
        Self {
            model_matrix: transform.local_matrix().to_cols_array_2d(),
        }
    }
}

pub struct CelestialBody {
    #[allow(dead_code)]
    name: String,
    texture: texture::Texture,
    mesh: mesh::Mesh,
    pub transform: transform::Transform,
    pub model_uniform: ModelUniform,
    pub model_buffer: wgpu::Buffer,
    pub model_bind_group: wgpu::BindGroup,
}
impl CelestialBody {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        texture: texture::Texture,
        transform: transform::Transform,
        model_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let mesh = mesh::Mesh::sphere(device, 128, 64);

        let model_uniform = ModelUniform::from_transform(&transform);

        let model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Model Buffer", name)),
            contents: bytemuck::cast_slice(&[model_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: model_buffer.as_entire_binding(),
            }],
            label: Some(&format!("{} Model Bind Group", name)),
        });

        Self {
            name: name.into(),
            texture,
            mesh,
            transform,
            model_uniform,
            model_buffer,
            model_bind_group,
        }
    }

    pub fn update(&mut self, dt: std::time::Duration) {
        // rotate 0.1 radians per second around the y-axis
        self.transform.rotation *= glam::Quat::from_rotation_y(0.1 * dt.as_secs_f32());
    }
}

pub trait DrawCelestialBody<'a> {
    fn draw_celestial_body(&mut self, cb: &'a CelestialBody, camera_bind_group: &wgpu::BindGroup);
    fn draw_celestial_body_instanced(
        &mut self,
        cb: &'a CelestialBody,
        instances: std::ops::Range<u32>,
        camera_bind_group: &wgpu::BindGroup,
    );
}
impl<'a, 'b: 'a> DrawCelestialBody<'b> for wgpu::RenderPass<'a> {
    fn draw_celestial_body(&mut self, cb: &'b CelestialBody, camera_bind_group: &wgpu::BindGroup) {
        self.draw_celestial_body_instanced(cb, 0..1, camera_bind_group);
    }

    fn draw_celestial_body_instanced(
        &mut self,
        cb: &'b CelestialBody,
        instances: std::ops::Range<u32>,
        camera_bind_group: &wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, cb.mesh.vertex_buffer.slice(..));
        self.set_index_buffer(cb.mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &cb.texture.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, &cb.model_bind_group, &[]);
        self.draw_indexed(0..cb.mesh.num_elements, 0, instances);
    }
}
