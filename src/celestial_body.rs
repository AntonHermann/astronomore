use wgpu::util::DeviceExt;

use crate::{mesh, texture};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    pub model_matrix: [[f32; 4]; 4],
}
impl ModelUniform {
    pub fn new(world_transform: &glam::Mat4, spin_transform: &glam::Mat4, radius: f32) -> Self {
        let scale = glam::Mat4::from_scale(glam::Vec3::splat(radius));
        let model_matrix = world_transform * spin_transform * scale;
        Self {
            model_matrix: model_matrix.to_cols_array_2d(),
        }
    }
}
impl Default for ModelUniform {
    fn default() -> Self {
        Self {
            model_matrix: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}

pub struct OrbitalParameters {
    /// Optional index of the parent celestial body in the scene's celestial_bodies list. None if this is a root body (e.g. the sun)
    pub parent_id: Option<usize>,
    /// Distance from the parent body
    /// TODO: unit?
    pub radius: f32,
    /// Angular velocity in radians per second
    pub angular_velocity: f32,
}

pub struct CelestialBody {
    #[allow(dead_code)]
    pub name: String,
    texture: texture::Texture,
    mesh: mesh::Mesh,
    pub radius: f32,
    pub orbital_parameters: OrbitalParameters,
    pub orbital_transform: glam::Mat4,
    pub spin_transform: glam::Mat4,
    pub model_uniform: ModelUniform,
    pub model_buffer: wgpu::Buffer,
    pub model_bind_group: wgpu::BindGroup,
}
impl CelestialBody {
    pub fn new(
        device: &wgpu::Device,
        name: &str,
        distance_from_parent: f32,
        radius: f32,
        angular_velocity: f32,
        texture: texture::Texture,
        model_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let mesh = mesh::Mesh::sphere(device, 128, 64);

        let orbital_parameters = OrbitalParameters {
            parent_id: None,
            radius: distance_from_parent,
            angular_velocity,
        };

        let model_uniform = ModelUniform::new(&glam::Mat4::IDENTITY, &glam::Mat4::IDENTITY, radius);

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
            radius,
            orbital_parameters,
            orbital_transform: glam::Mat4::IDENTITY,
            spin_transform: glam::Mat4::IDENTITY,
            model_uniform,
            model_buffer,
            model_bind_group,
        }
    }

    pub fn update(&mut self, sim_time: f64) {
        let sim_time = sim_time as f32;
        let angle = self.orbital_parameters.angular_velocity * sim_time;
        let pos = glam::Vec3::new(
            self.orbital_parameters.radius * angle.cos(),
            0.,
            self.orbital_parameters.radius * angle.sin(),
        );
        self.orbital_transform = glam::Mat4::from_translation(pos);

        self.spin_transform = glam::Mat4::from_rotation_y(0.1 * sim_time);
    }

    /// Update the model uniform based on its absolute world transform (i.e. combined with parent transforms) and its spin transform
    pub fn update_model_uniform(&mut self, world_transform: glam::Mat4) {
        self.model_uniform = ModelUniform::new(&world_transform, &self.spin_transform, self.radius);
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
