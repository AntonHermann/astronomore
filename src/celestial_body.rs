use crate::{mesh, texture};

pub struct CelestialBody {
    #[allow(dead_code)]
    name: String,
    texture: texture::Texture,
    mesh: mesh::Mesh,
}
impl CelestialBody {
    pub fn new(device: &wgpu::Device, name: &str, texture: texture::Texture) -> Self {
        let mesh = mesh::Mesh::sphere(device, 128, 64);

        Self {
            name: name.into(),
            texture,
            mesh,
        }
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
        self.draw_indexed(0..cb.mesh.num_elements, 0, instances);
    }
}
