use crate::celestial_body::CelestialBody;

pub struct Scene {
    pub celestial_bodies: Vec<CelestialBody>,
    pub model_bind_group_layout: wgpu::BindGroupLayout,
}

impl Scene {
    pub fn new(device: &wgpu::Device) -> Self {
        let model_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // only the vertex shader needs to see this
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("model_bind_group_layout"),
            });
        Self {
            celestial_bodies: Vec::new(),
            model_bind_group_layout,
        }
    }

    pub fn add_celestial_body(&mut self, body: CelestialBody) {
        self.celestial_bodies.push(body);
    }

    pub fn update(&mut self, dt: std::time::Duration, queue: &wgpu::Queue) {
        // update celestial bodies (including their local transforms)
        for cb in &mut self.celestial_bodies {
            cb.update(dt);
        }

        // calculate global transforms and update model uniforms for all celestial bodies
        // assumtion: parents are always before their children in the list, so we can just iterate through the list and update each celestial body
        let mut model_matrices = Vec::with_capacity(self.celestial_bodies.len());
        for i in 0..self.celestial_bodies.len() {
            let local_matrix = self.celestial_bodies[i].transform.local_matrix();
            let world_matrix = match self.celestial_bodies[i].transform.parent_id {
                Some(parent_id) => model_matrices[parent_id] * local_matrix,
                None => local_matrix,
            };
            model_matrices.push(world_matrix);

            // update the model uniform for this celestial body
            self.celestial_bodies[i].model_uniform.model_matrix = world_matrix.to_cols_array_2d();
            // write the updated model uniform to the GPU
            queue.write_buffer(
                &self.celestial_bodies[i].model_buffer,
                0,
                bytemuck::cast_slice(&[self.celestial_bodies[i].model_uniform]),
            );
        }
    }
}
